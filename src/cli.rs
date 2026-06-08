use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "pv", version, about = "Package and version manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Install(PackageArg),
    Uninstall(PackageArg),
    Search(SearchArg),
    List(ListArg),
    Use(ExactPackageArg),
    Shell(ExactPackageArg),
    Info(InfoArg),
    Outdated,
    Upgrade(UpgradeArg),
    Sync,
    Bucket(BucketCommand),
    /// 管理下载缓存
    Cache(CacheCommand),
    /// 清理旧版本
    Cleanup(CleanupArg),
    /// 列出远程可用版本
    LsRemote(LsRemoteArg),
    /// 显示安装路径
    Where(WhereArg),
}

#[derive(Debug, Args)]
pub struct PackageArg {
    pub package: String,
}

#[derive(Debug, Args)]
pub struct ExactPackageArg {
    pub package: String,
}

#[derive(Debug, Args)]
pub struct SearchArg {
    pub keyword: String,
}

#[derive(Debug, Args)]
pub struct ListArg {
    pub package: Option<String>,
}

#[derive(Debug, Args)]
pub struct InfoArg {
    pub package: String,
}

#[derive(Debug, Args)]
pub struct UpgradeArg {
    pub package: Option<String>,
}

// ── Bucket ──

#[derive(Debug, Subcommand)]
pub enum BucketSubcommand {
    Add(BucketAddArg),
    List,
    Rm(BucketRemoveArg),
}

#[derive(Debug, Args)]
pub struct BucketCommand {
    #[command(subcommand)]
    pub command: BucketSubcommand,
}

#[derive(Debug, Args)]
pub struct BucketAddArg {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Args)]
pub struct BucketRemoveArg {
    pub name: String,
}

// ── Cache ──

#[derive(Debug, Subcommand)]
pub enum CacheSubcommand {
    /// 显示缓存大小和路径
    Show,
    /// 清空所有缓存
    Clean,
}

#[derive(Debug, Args)]
pub struct CacheCommand {
    #[command(subcommand)]
    pub command: CacheSubcommand,
}

// ── Cleanup ──

#[derive(Debug, Args)]
pub struct CleanupArg {
    /// 只清理指定包
    pub package: Option<String>,
    /// 预览模式，不实际删除
    #[arg(long)]
    pub dry_run: bool,
}

// ── LsRemote ──

#[derive(Debug, Args)]
pub struct LsRemoteArg {
    /// 包名，支持 npm: 前缀
    pub package: String,
}

// ── Where ──

#[derive(Debug, Args)]
pub struct WhereArg {
    /// 包名
    pub package: String,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let paths = crate::config::Paths::discover()?;
    let config = crate::config::Config::load_or_default(&paths)?;
    #[cfg(windows)]
    let platform = crate::platform::windows::WindowsPlatform;
    #[cfg(not(windows))]
    let platform = UnsupportedCliPlatform;
    let mut engine = crate::engine::Engine::new(paths.clone(), config, &platform);

    match cli.command {
        Commands::Install(arg) => {
            engine.install(&arg.package).await?;
            println!("installed {}", arg.package);
        }
        Commands::Uninstall(arg) => {
            engine.uninstall(&arg.package)?;
            println!("uninstalled {}", arg.package);
        }
        Commands::Search(arg) => {
            for result in engine.search(&arg.keyword)? {
                println!(
                    "{} {} {}",
                    result.name,
                    result.version,
                    result.description.unwrap_or_default()
                );
            }
        }
        Commands::List(arg) => {
            for listing in engine.list(arg.package.as_deref())? {
                for version in listing.versions {
                    let marker = if version.active { "*" } else { " " };
                    println!("{marker} {} {}", listing.package, version.version);
                }
            }
        }
        Commands::Use(arg) => {
            engine.use_version(&arg.package)?;
            println!("using {}", arg.package);
        }
        Commands::Shell(arg) => {
            engine.shell(&arg.package)?;
        }
        Commands::Info(arg) => {
            let manifest = engine.info(&arg.package)?;
            println!("{} {}", manifest.name, manifest.version);
            if let Some(description) = manifest.description {
                println!("{description}");
            }
        }
        Commands::Outdated => {
            for item in engine.outdated()? {
                println!("{} {} -> {}", item.package, item.installed, item.available);
            }
        }
        Commands::Upgrade(arg) => {
            engine.upgrade(arg.package.as_deref()).await?;
            println!("upgrade complete");
        }
        Commands::Sync => {
            let manager = crate::bucket::BucketManager::new(paths, engine.config().clone());
            manager.sync()?;
            println!("sync complete");
        }
        Commands::Bucket(arg) => handle_bucket_command(arg).await?,
        Commands::Cache(arg) => match arg.command {
            CacheSubcommand::Show => engine.cache_show()?,
            CacheSubcommand::Clean => engine.cache_clean()?,
        },
        Commands::Cleanup(arg) => engine.cleanup(arg.package.as_deref(), arg.dry_run)?,
        Commands::LsRemote(arg) => {
            for version in engine.ls_remote(&arg.package)? {
                println!("{version}");
            }
        }
        Commands::Where(arg) => engine.where_is(&arg.package)?,
    }

    Ok(())
}

async fn handle_bucket_command(arg: BucketCommand) -> Result<()> {
    let paths = crate::config::Paths::discover()?;
    let config = crate::config::Config::load_or_default(&paths)?;
    match arg.command {
        BucketSubcommand::Add(add) => {
            let mut manager = crate::bucket::BucketManager::new(paths.clone(), config);
            manager.add(&add.name, &add.url)?;
            manager.config().save(&paths)?;
            println!("bucket added {}", add.name);
        }
        BucketSubcommand::List => {
            for bucket in &config.buckets {
                println!("{} {}", bucket.name, bucket.url);
            }
        }
        BucketSubcommand::Rm(remove) => {
            let mut manager = crate::bucket::BucketManager::new(paths.clone(), config);
            manager.remove(&remove.name)?;
            manager.config().save(&paths)?;
            println!("bucket removed {}", remove.name);
        }
    }
    Ok(())
}

#[cfg(not(windows))]
struct UnsupportedCliPlatform;

#[cfg(not(windows))]
impl crate::platform::Platform for UnsupportedCliPlatform {
    fn make_active_link(
        &self,
        _target: &std::path::Path,
        _link: &std::path::Path,
    ) -> crate::error::Result<()> {
        Err(crate::error::PvError::UnsupportedPlatform(
            "Windows MVP only".to_string(),
        ))
    }

    fn remove_active_link(&self, _link: &std::path::Path) -> crate::error::Result<()> {
        Ok(())
    }

    fn create_shim(
        &self,
        _exe_name: &str,
        _shim_exe: &std::path::Path,
        _shim_config: &std::path::Path,
        _target: &std::path::Path,
    ) -> crate::error::Result<()> {
        Err(crate::error::PvError::UnsupportedPlatform(
            "Windows MVP only".to_string(),
        ))
    }

    fn remove_shim(
        &self,
        _shim_exe: &std::path::Path,
        _shim_config: &std::path::Path,
    ) -> crate::error::Result<()> {
        Ok(())
    }

    fn register_path(&self, _dir: &std::path::Path) -> crate::error::Result<()> {
        Ok(())
    }

    fn exe_ext(&self) -> &'static str {
        ""
    }

    fn spawn_shell_with_path(&self, _path_prefix: &std::path::Path) -> crate::error::Result<()> {
        Err(crate::error::PvError::UnsupportedPlatform(
            "Windows MVP only".to_string(),
        ))
    }
}
