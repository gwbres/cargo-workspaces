mod cargo;
mod changable;
mod config;
mod dag;
mod dev_dep_remover;
mod error;
mod git;
mod listable;
mod pkg;
mod version;

pub use cargo::{
    cargo, cargo_config_get, change_versions, check_index, is_published, rename_packages,
};
pub use changable::{ChangeData, ChangeOpt};
pub use config::{read_config, PackageConfig, WorkspaceConfig};
pub use dag::dag;
pub use dev_dep_remover::{should_remove_dev_deps, DevDependencyRemover};
pub(crate) use error::{debug, info, warn};
pub use error::{get_debug, set_debug, Error};
pub use git::{git, GitOpt};
pub use listable::{ListOpt, Listable};
pub use pkg::{get_pkgs, Pkg};
pub use version::VersionOpt;

pub type Result<T = ()> = std::result::Result<T, Error>;

pub const INTERNAL_ERR: &str = "Internal error message. Please create an issue on https://github.com/pksunkara/cargo-workspaces";

pub fn validate_value_containing_name(value: &str) -> std::result::Result<(), String> {
    if !value.contains("%n") {
        return Err("must contain '%n'\n".to_string());
    }

    Ok(())
}
