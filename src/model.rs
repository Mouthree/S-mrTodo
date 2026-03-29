use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::storage::{GetId2Key};

//列表id,第一个是上级id,第二个是自己的
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize, Hash, Default)]
pub struct ListId {
    //上级id
    pub parent: u64,
    //自己id
    pub own: u64,
}

//条目id,同上
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize, Hash, Default)]
pub struct ItemId {
    pub parent: u64,
    pub own: u64,
}

//拼接键
pub fn encode_key(parent: u64, own: u64) -> [u8; 16] {
    let mut key = [0u8; 16];
    key[0..8].copy_from_slice(&parent.to_be_bytes());
    key[8..16].copy_from_slice(&own.to_be_bytes());
    key
}

//列表结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoList {
    pub id: ListId,
    pub name: String,
    pub note: Option<String>,
    pub tags: Vec<String>,
}
impl GetId2Key for TodoList {
    fn get_key(&self) -> [u8; 16] {
        encode_key(self.id.parent, self.id.own)
    }
}

//条目结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    //本条id(这个id是由本体唯一id和父id组成, 前面是父,后面是子)
    pub id: ItemId,
    //条目名字
    pub title: String,
    //条目备注
    pub note: Option<String>,
    //条目的具体数据或类型
    pub main_data: ItemVariant,
    //完成情况
    pub current_state: Option<CurrentState>,
    //开始时间
    pub start_time: NaiveDateTime,
    //设定的完成时间
    pub dead_line: Option<NaiveDateTime>,
    //优先级
    pub priority: Option<Priority> 
}
impl GetId2Key for Item {
    fn get_key(&self) -> [u8; 16] {
        encode_key(self.id.parent, self.id.own)
    }
}
//设定的优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct  Priority {
    //实际优先级
    pub level: u8,
    //显示的名字,可以自定义优先级
    pub label: String
}

//完成情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurrentState {
    //未开始
    NotStart,
    //工作中,这里有两个变体,可选显示进度或者不显示进度
    Working(IfProgress),
    //已结束
    Over
}
//可选是否有进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IfProgress {
    No,
    Progress(f32)
}
//条目的变体
//每个条目可以是一个基本条目,或者一个列表,或者一个任务条目,这里使用枚举方便以后扩展功能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemVariant {
    //基本条目
    Basic,
    //任务条目
    Command {
        task: Task
    },
    //列表
    List {
        list_id: ListId
    }
}
//这里到时候需要写任务细节
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {

}
