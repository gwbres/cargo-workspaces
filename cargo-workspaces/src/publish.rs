use crate::utils::{
    cargo, cargo_config_get, check_index, dag, info, is_published, should_remove_dev_deps, warn,
    DevDependencyRemover, Error, Result, VersionOpt, INTERNAL_ERR,
};
use cargo_metadata::Metadata;
use clap::Parser;
use crates_index::Index;
use indexmap::IndexSet as Set;

/// Publish crates in the project
#[derive(Debug, Parser)]
#[clap(next_help_heading = "PUBLISH OPTIONS")]
pub struct Publish {
    #[clap(flatten, next_help_heading = None)]
    version: VersionOpt,

    /// Publish crates from the current commit without versioning
    // TODO: conflicts_with = "version" (group)
    #[clap(long)]
    from_git: bool,

    /// Skip already published crate versions
    #[clap(long, hide = true)]
    skip_published: bool,

    /// Skip crate verification (not recommended)
    #[clap(long)]
    no_verify: bool,

    /// Allow dirty working directories to be published
    #[clap(long)]
    allow_dirty: bool,

    /// The token to use for publishing
    #[clap(long, forbid_empty_values(true))]
    token: Option<String>,

    /// The Cargo registry to use for publishing
    #[clap(long, forbid_empty_values(true))]
    registry: Option<String>,

    /// Don't remove dev-dependencies while publishing
    #[clap(long)]
    no_remove_dev_deps: bool,
}

impl Publish {
    pub fn run(self, metadata: Metadata) -> Result {
        let pkgs = if !self.from_git {
            self.version
                .do_versioning(&metadata)?
                .iter()
                .map(|x| {
                    (
                        metadata
                            .packages
                            .iter()
                            .find(|y| x.0 == &y.name)
                            .expect(INTERNAL_ERR)
                            .clone(),
                        x.1.to_string(),
                    )
                })
                .collect::<Vec<_>>()
        } else {
            metadata
                .packages
                .iter()
                .map(|x| (x.clone(), x.version.to_string()))
                .collect()
        };

        let (names, visited) = dag(&pkgs);

        // Filter out private packages
        let visited = visited
            .into_iter()
            .filter(|x| {
                if let Some((pkg, _)) = pkgs.iter().find(|(p, _)| p.manifest_path == *x) {
                    return pkg.publish.is_none()
                        || !pkg.publish.as_ref().expect(INTERNAL_ERR).is_empty();
                }

                false
            })
            .collect::<Set<_>>();

        for p in &visited {
            let (pkg, version) = names.get(p).expect(INTERNAL_ERR);
            let name = pkg.name.clone();
            let mut args = vec!["publish"];

            let name_ver = format!("{} v{}", name, version);

            let mut index = if let Some(registry) = self
                .registry
                .as_ref()
                .or_else(|| pkg.publish.as_deref().and_then(|x| x.get(0)))
            {
                let registry_url = cargo_config_get(
                    &metadata.workspace_root,
                    &format!("registries.{}.index", registry),
                )?;
                Index::from_url(&format!("registry+{}", registry_url))?
            } else {
                Index::new_cargo_default()?
            };

            if is_published(&mut index, &name, version)? {
                info!("already published", name_ver);
                continue;
            }

            if self.no_verify {
                args.push("--no-verify");
            }

            if self.allow_dirty {
                args.push("--allow-dirty");
            }

            if let Some(ref registry) = self.registry {
                args.push("--registry");
                args.push(registry);
            }

            if let Some(ref token) = self.token {
                args.push("--token");
                args.push(token);
            }

            args.push("--manifest-path");
            args.push(p.as_str());

            let dev_deps_remover =
                if self.no_remove_dev_deps || !should_remove_dev_deps(&pkg.dependencies, &pkgs) {
                    None
                } else {
                    warn!(
                        "removing dev-deps since some refer to workspace members with versions",
                        name_ver
                    );
                    Some(DevDependencyRemover::remove_dev_deps(p.as_std_path())?)
                };

            let (_, stderr) = cargo(&metadata.workspace_root, &args, &[])?;

            drop(dev_deps_remover);

            if !stderr.contains("Uploading") || stderr.contains("error:") {
                return Err(Error::Publish(name));
            }

            check_index(&mut index, &name, version)?;

            info!("published", name_ver);
        }

        info!("success", "ok");
        Ok(())
    }
}
