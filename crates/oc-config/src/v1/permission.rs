//! Ported from: packages/core/src/v1/config/permission.ts

use serde::{Deserialize, Serialize};

use super::OrderedMap;

/// Ported from: packages/core/src/v1/config/permission.ts:5-6 (Action)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionAction {
    Ask,
    Allow,
    Deny,
}

impl PermissionAction {
    pub fn as_str(self) -> &'static str {
        match self {
            PermissionAction::Ask => "ask",
            PermissionAction::Allow => "allow",
            PermissionAction::Deny => "deny",
        }
    }
}

fn action_from_str(value: &str) -> Option<PermissionAction> {
    match value {
        "ask" => Some(PermissionAction::Ask),
        "allow" => Some(PermissionAction::Allow),
        "deny" => Some(PermissionAction::Deny),
        _ => None,
    }
}

/// Ported from: packages/core/src/v1/config/permission.ts:11-12 (Rule = Action | Object)
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PermissionRule {
    Action(PermissionAction),
    Object(OrderedMap<PermissionAction>),
}

impl<'de> Deserialize<'de> for PermissionRule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => match action_from_str(&s) {
                Some(action) => Ok(PermissionRule::Action(action)),
                None => Err(serde::de::Error::custom(
                    "Expected permission action ask|allow|deny or rule object",
                )),
            },
            other @ serde_json::Value::Object(_) => {
                serde_json::from_value::<OrderedMap<PermissionAction>>(other)
                    .map(PermissionRule::Object)
                    .map_err(serde::de::Error::custom)
            }
            _ => Err(serde::de::Error::custom(
                "Expected permission action string or rule object",
            )),
        }
    }
}

/// Ported from: packages/core/src/v1/config/permission.ts:17-36 (InputObject)
/// StructWithRest: key dikenal bertipe + flatten Record(String, Rule).
#[derive(Debug, Clone, Serialize)]
pub struct PermissionInputObject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read: Option<PermissionRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit: Option<PermissionRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glob: Option<PermissionRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grep: Option<PermissionRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list: Option<PermissionRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bash: Option<PermissionRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<PermissionRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_directory: Option<PermissionRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todowrite: Option<PermissionAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<PermissionAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webfetch: Option<PermissionAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub websearch: Option<PermissionAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lsp: Option<PermissionRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doom_loop: Option<PermissionAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<PermissionRule>,
    #[serde(flatten)]
    pub rest: OrderedMap<PermissionRule>,
}

impl<'de> Deserialize<'de> for PermissionInputObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let value = serde_json::Value::deserialize(deserializer)?;
        let serde_json::Value::Object(map) = value else {
            return Err(Error::custom("Expected permission object"));
        };
        let take_rule = |map: &serde_json::Map<String, serde_json::Value>,
                         key: &str|
         -> Result<Option<PermissionRule>, D::Error> {
            match map.get(key).filter(|v| !v.is_null()) {
                Some(v) => Ok(Some(
                    serde_json::from_value::<PermissionRule>(v.clone()).map_err(Error::custom)?,
                )),
                None => Ok(None),
            }
        };
        let take_action = |map: &serde_json::Map<String, serde_json::Value>,
                           key: &str|
         -> Result<Option<PermissionAction>, D::Error> {
            match map.get(key).filter(|v| !v.is_null()) {
                Some(v) => Ok(Some(
                    serde_json::from_value::<PermissionAction>(v.clone()).map_err(Error::custom)?,
                )),
                None => Ok(None),
            }
        };
        let read = take_rule(&map, "read")?;
        let edit = take_rule(&map, "edit")?;
        let glob = take_rule(&map, "glob")?;
        let grep = take_rule(&map, "grep")?;
        let list = take_rule(&map, "list")?;
        let bash = take_rule(&map, "bash")?;
        let task = take_rule(&map, "task")?;
        let external_directory = take_rule(&map, "external_directory")?;
        let todowrite = take_action(&map, "todowrite")?;
        let question = take_action(&map, "question")?;
        let webfetch = take_action(&map, "webfetch")?;
        let websearch = take_action(&map, "websearch")?;
        let lsp = take_rule(&map, "lsp")?;
        let doom_loop = take_action(&map, "doom_loop")?;
        let skill = take_rule(&map, "skill")?;

        let mut rest: OrderedMap<PermissionRule> = OrderedMap::new();
        for (key, value) in map {
            if PERMISSION_KNOWN_KEYS.contains(&key.as_str()) {
                continue;
            }
            if value.is_null() {
                continue;
            }
            let rule = serde_json::from_value::<PermissionRule>(value).map_err(Error::custom)?;
            rest.insert(key, rule);
        }
        Ok(PermissionInputObject {
            read,
            edit,
            glob,
            grep,
            list,
            bash,
            task,
            external_directory,
            todowrite,
            question,
            webfetch,
            websearch,
            lsp,
            doom_loop,
            skill,
            rest,
        })
    }
}

impl Default for PermissionInputObject {
    fn default() -> Self {
        PermissionInputObject {
            read: None,
            edit: None,
            glob: None,
            grep: None,
            list: None,
            bash: None,
            task: None,
            external_directory: None,
            todowrite: None,
            question: None,
            webfetch: None,
            websearch: None,
            lsp: None,
            doom_loop: None,
            skill: None,
            rest: OrderedMap::new(),
        }
    }
}

const PERMISSION_KNOWN_KEYS: &[&str] = &[
    "read",
    "edit",
    "glob",
    "grep",
    "list",
    "bash",
    "task",
    "external_directory",
    "todowrite",
    "question",
    "webfetch",
    "websearch",
    "lsp",
    "doom_loop",
    "skill",
];

/// Ported from: packages/core/src/v1/config/permission.ts:38-48
/// (Info = Union(Action, InputObject) + normalizeInput string→{"*": action})
#[derive(Debug, Clone, Default, Serialize)]
pub struct PermissionInfo {
    #[serde(flatten)]
    pub object: PermissionInputObject,
}

impl<'de> Deserialize<'de> for PermissionInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(ref s) => {
                let action = action_from_str(s)
                    .ok_or_else(|| serde::de::Error::custom("Expected permission action"))?;
                let mut rest: OrderedMap<PermissionRule> = OrderedMap::new();
                rest.insert("*".to_string(), PermissionRule::Action(action));
                Ok(PermissionInfo {
                    object: PermissionInputObject {
                        rest,
                        ..Default::default()
                    },
                })
            }
            other => {
                let object: PermissionInputObject =
                    serde_json::from_value(other).map_err(serde::de::Error::custom)?;
                Ok(PermissionInfo { object })
            }
        }
    }
}

impl PermissionInfo {
    /// PermissionInputObject kosong `{}`.
    pub fn empty() -> Self {
        PermissionInfo {
            object: PermissionInputObject::default(),
        }
    }

    pub fn set_edit(&mut self, action: &str) {
        self.object.edit = Some(PermissionRule::Action(parse_action(action)));
    }

    pub fn insert_rule(&mut self, tool: String, action: &str) {
        self.object
            .rest
            .insert(tool, PermissionRule::Action(parse_action(action)));
    }

    /// Meniru `Object.assign(permission, agent.permission)` — hanya key yang
    /// ADA di sumber yang menimpa; sisanya dibiarkan.
    pub fn assign_from(&mut self, other: PermissionInfo) {
        let src = other.object;
        if src.read.is_some() {
            self.object.read = src.read;
        }
        if src.edit.is_some() {
            self.object.edit = src.edit;
        }
        if src.glob.is_some() {
            self.object.glob = src.glob;
        }
        if src.grep.is_some() {
            self.object.grep = src.grep;
        }
        if src.list.is_some() {
            self.object.list = src.list;
        }
        if src.bash.is_some() {
            self.object.bash = src.bash;
        }
        if src.task.is_some() {
            self.object.task = src.task;
        }
        if src.external_directory.is_some() {
            self.object.external_directory = src.external_directory;
        }
        if src.todowrite.is_some() {
            self.object.todowrite = src.todowrite;
        }
        if src.question.is_some() {
            self.object.question = src.question;
        }
        if src.webfetch.is_some() {
            self.object.webfetch = src.webfetch;
        }
        if src.websearch.is_some() {
            self.object.websearch = src.websearch;
        }
        if src.lsp.is_some() {
            self.object.lsp = src.lsp;
        }
        if src.doom_loop.is_some() {
            self.object.doom_loop = src.doom_loop;
        }
        if src.skill.is_some() {
            self.object.skill = src.skill;
        }
        self.object.rest.assign_from(src.rest);
    }
}

fn parse_action(action: &str) -> PermissionAction {
    action_from_str(action).unwrap_or(PermissionAction::Ask)
}
