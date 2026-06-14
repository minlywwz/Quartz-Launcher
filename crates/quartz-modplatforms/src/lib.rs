pub mod curseforge;
pub mod modrinth;
pub mod mrpack;

pub use curseforge::{CurseForgeClient, CurseForgeError, ModpackHit as CurseForgeModpackHit};
pub use modrinth::{
    ModrinthClient, ModrinthError, ModrinthGameVersion, ModrinthHit, ModrinthSearchResponse,
    ModrinthVersion,
};
pub use mrpack::{
    download_and_install_mrpack, install_mrpack, MrpackError, MrpackInstallResult,
};
