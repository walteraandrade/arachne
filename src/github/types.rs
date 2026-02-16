#[derive(Clone, Debug)]
pub struct ForkInfo {
    pub owner: String,
    pub repo: String,
    #[allow(dead_code)]
    pub default_branch: String,
}
