use parking_lot::RwLock;
use std::{collections::HashMap, error, hash::Hash, path::Path};

use serde::{Serialize, de::DeserializeOwned};
use sled::Db;
use thiserror::Error;

use crate::model::{Item, ItemId, ListId, TodoList, encode_key};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("数据库操作失败: {0}")]
    Sled(#[from] sled::Error),

    #[error("序列化/逆序列化失败: {0}")]
    Postcard(#[from] postcard::Error),

    #[error("数据不存在: parent: {parent} | own: {own}")]
    NotFound{ parent: u64, own: u64 },

    #[error("唯一读报错")]
    LockErr
    
}

//用来获取结构体中的id对,直接用来作为数据库的键
pub trait GetId2Key {
    fn get_key(&self) -> [u8; 16];
}

//用来判断需要返回哪种列表
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListType {
    All, 
    OnlyRoot,
    OnlyNonRoot, 
}

pub struct Storage {
    list_tree: sled::Tree,
    item_tree: sled::Tree,
    db: Db,
    list_cache: RwLock<HashMap<[u8; 16], TodoList>>
}
impl Storage {
    //:) 打开数据库文件
    //初始化的时候获取到所有的列表项,存到列表中
    //显示的时候默认显示根节点,至于显示别的list需要另一个指令
    pub fn new(path: &Path) -> Result<Self, StorageError> {
        let db = sled::open(path)?;
        let list_tree = db.open_tree("list")?;
        let item_tree = db.open_tree("item")?;
        //初始化缓存
        let mut list_cache = HashMap::new();
        //获取数据库中所有的list
        for res in list_tree.iter() {
            let (key, value) = res?;
            let list: TodoList = postcard::from_bytes(&value)?;
            list_cache.insert(list.get_key(), list);
        }
        Ok(Self { list_tree, item_tree, db, list_cache: RwLock::new(list_cache)})
    }
    //:) 生成唯一list的id
    pub fn create_list_id(&self, parent: u64) -> Result<ListId, StorageError> {
        let own = self.db.generate_id()?;
        Ok(ListId{
            parent,
            own
        })
    }
    //:) 生成唯一item的id
    pub fn create_item_id(&self, parent: u64) -> Result<ItemId, StorageError> {
        let own = self.db.generate_id()?;
        Ok(ItemId{
            parent,
            own
        })
    }
    //:) 创建 | 修改列表
    pub fn save_list(&self, data: &TodoList) -> Result<(), StorageError> {
        //序列化
        let bytes = postcard::to_allocvec(data)?;
        //存数据库
        let key = data.get_key();
        self.list_tree.insert(key, bytes)?;
        //写缓存
        let mut cache = self.list_cache.write();
        cache.insert(key, data.clone());
        Ok(())
    }
    //:) 创建 | 修改项
    pub fn save_item(&self, data: &Item) -> Result<(), StorageError> {
        //序列化
        let bytes = postcard::to_allocvec(data)?;
        //存
        let key = data.get_key();
        self.item_tree.insert(key, bytes)?;
        Ok(())
    }
    //:) 获取所有的列表,通过list_type判断是否是根节点
    pub fn get_all_list(&self, list_type: ListType) -> Result<Vec<TodoList>, StorageError> {
        let cache = self.list_cache.read();
        let lists: Vec<TodoList> = cache.values().filter(|list| {
            match list_type {
                ListType::All => true,
                ListType::OnlyRoot => list.id.parent == 0,
                ListType::OnlyNonRoot => list.id.parent != 0
            }
        }).cloned().collect();
        Ok(lists)
    }
    //:) 读list,优先读缓存
    pub fn get_list(&self, id: ListId) -> Result<TodoList, StorageError> {
        let key = encode_key(id.parent, id.own);
        //先在缓存里面找
        if let Some(list) = self.list_cache.read().get(&key) {
            return Ok(list.clone());
        }
        //万一真就找不到了再去数据库查
        let t = self.list_tree.get(key)?;
        match t {
            Some(t) => {
                let t: TodoList = postcard::from_bytes(&t)?;
                let mut cache = self.list_cache.write();
                //存到缓存里
                cache.insert(key, t.clone());
                Ok(t)
            },
            None => {
                //数据库里面也没有就报错
                Err(StorageError::NotFound{parent: id.parent, own: id.own})
            }
        }
    }
    //:) 读item
    pub fn get_item(&self, id: ItemId) -> Result<Item, StorageError> {
        let key = encode_key(id.parent, id.own);
        let t = self.item_tree.get(key)?;
        match t {
            Some(t) => {
                let t = postcard::from_bytes(&t)?;
                Ok(t)
            },
            None => {
                Err(StorageError::NotFound{parent: id.parent, own: id.own})
            }
        }
    }
    //:) 获取list下所有item
    pub fn get_items_of_list(&self, id: ListId) -> Result<Vec<Item>, StorageError> {
        let list_id = id.own.to_be_bytes();
        let mut items: Vec<Item> = Vec::new();
        for i in self.item_tree.scan_prefix(list_id) {
            let (_, v) = i?;
            let item: Item = postcard::from_bytes(&v)?;
            items.push(item);
        }
        Ok(items)
    }
    //:) 删除item
    pub fn delete_item(&self, id: ItemId) -> Result<(), StorageError> {
        let key = encode_key(id.parent, id.own);
        self.item_tree.remove(key)?;
        Ok(())
    }
    //:) 删除list
    pub fn delete_list(&self, id: ListId) -> Result<(), StorageError> {
        let list_key = encode_key(id.parent, id.own);
        let list_own_key = id.own.to_be_bytes();
        let mut batch = sled::Batch::default();
        //删除下面的所有item
        //这里先统一收集再一起删除,要成功就一起成功,要失败就一起失败
        for i in self.item_tree.scan_prefix(list_own_key) {
            let i = i?;
            batch.remove(i.0);
        }
        self.item_tree.apply_batch(batch)?;
        //删除自己
        self.list_tree.remove(list_key)?;
        //删除缓存
        self.list_cache.write().remove(&list_key);
        Ok(())
    }
}

