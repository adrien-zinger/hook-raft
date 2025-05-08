use crate::{
    api::io_msg::UpdateNodeResult,
    common::{error::WarnResult, Url},
    Node, Settings,
};

impl Node {
    pub async fn send_init_update_node(
        &self,
        target: &Url,
        settings: &Settings,
        uuid: [u8; 16],
    ) -> WarnResult<UpdateNodeResult> {
        Ok(UpdateNodeResult {
            leader_id: String::default(),
            node_list: Vec::<String>::default(),
        })
    }
}
