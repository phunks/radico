use include_assets::EnumArchive;
use std::ops::Index;
use std::sync::LazyLock;

#[allow(clippy::redundant_closure)]
pub static ASSETS: LazyLock<Archive> = LazyLock::new(|| Archive::new());
#[derive(include_assets::AssetEnum)]
#[archive(base_path = "assets")]
pub enum Asset {
    #[asset(path = "n001.aac")]
    N001,
    #[asset(path = "n002.aac")]
    N002,
    #[asset(path = "n003.aac")]
    N003,
    #[asset(path = "n004.aac")]
    N004,
    #[asset(path = "n005.aac")]
    N005,
    #[asset(path = "n006.aac")]
    N006,
}

pub struct Archive(pub EnumArchive<Asset>);
impl Archive {
    pub fn new() -> Self {
        Self(EnumArchive::<Asset>::load())
    }

    pub fn get(&self, i: usize) -> Vec<u8> {
        self.0.index(i.into()).to_vec()
    }
}

impl From<usize> for Asset {
    fn from(num: usize) -> Self {
        match num {
            0 => Asset::N001,
            1 => Asset::N002,
            2 => Asset::N003,
            3 => Asset::N004,
            4 => Asset::N005,
            5 => Asset::N006,
            _ => panic!("invalid value"),
        }
    }
}
