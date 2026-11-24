use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    /// YYYY-MM-DD. 없으면 "언젠가 할 일"
    pub date: Option<String>,
    /// HH:MM
    pub due_time: Option<String>,
    pub tag_id: Option<String>,
    pub done: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 할 일 추가/수정 시 화면에서 보내는 값 (F-7: 제목만 필수)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub date: Option<String>,
    pub due_time: Option<String>,
    pub tag_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    /// 팔레트 키 (purple, yellow, sky, orange, green, pink, teal, gray)
    pub color: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagInput {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// 마감 알림 전체 켜기/끄기 (F-20)
    pub notifications_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { notifications_enabled: true }
    }
}
