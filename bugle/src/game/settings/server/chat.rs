use ini_persist::load::LoadProperty;

#[derive(Debug, Clone, LoadProperty)]
pub struct ChatSettings {
    #[ini(rename = "ChatLocalRadius")]
    pub local_radius: f64,

    #[ini(rename = "ChatMaxMessageLength")]
    pub max_msg_len: u16,

    #[ini(rename = "ChatHasGlobal")]
    pub global_enabled: bool,
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self {
            local_radius: 5000.0,
            max_msg_len: 512,
            global_enabled: true,
        }
    }
}
