use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};

#[allow(dead_code)]
pub struct PluginContext {
    pub server_version: String,
    pub data_folder: PathBuf,
}

#[allow(dead_code)]
pub trait CrabPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str {
        "A CrabMC Plugin"
    }
    fn on_enable(&mut self, ctx: &PluginContext) -> anyhow::Result<()>;
    fn on_disable(&mut self) -> anyhow::Result<()>;
    fn on_command(&mut self, _sender: &str, _command: &str, _args: &[&str]) -> bool {
        false
    }
}

pub struct CrabEssentialsPlugin;

impl CrabPlugin for CrabEssentialsPlugin {
    fn name(&self) -> &str {
        "CrabEssentials"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn description(&self) -> &str {
        "Core essentials plugin for CrabMC server (Rust port of PaperMC concepts)"
    }

    fn on_enable(&mut self, ctx: &PluginContext) -> anyhow::Result<()> {
        info!(
            "[{}] Enabled successfully on CrabMC {}!",
            self.name(),
            ctx.server_version
        );
        info!("[{}] Ready to process player commands and events.", self.name());
        Ok(())
    }

    fn on_disable(&mut self) -> anyhow::Result<()> {
        info!("[{}] Disabled. Saving plugin state...", self.name());
        Ok(())
    }

    fn on_command(&mut self, sender: &str, command: &str, args: &[&str]) -> bool {
        match command {
            "ping" => {
                info!("[CrabEssentials] {} executed /ping -> Pong!", sender);
                true
            }
            "crab" => {
                info!(
                    "[CrabEssentials] 🦀 CrabMC is running smoothly with Rust! Args: {:?}",
                    args
                );
                true
            }
            _ => false,
        }
    }
}

pub struct PluginManager {
    plugins: Vec<Box<dyn CrabPlugin>>,
    plugins_dir: PathBuf,
}

impl PluginManager {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        let plugins_dir = dir.as_ref().to_path_buf();
        if !plugins_dir.exists() {
            let _ = fs::create_dir_all(&plugins_dir);
        }

        Self {
            plugins: Vec::new(),
            plugins_dir,
        }
    }

    pub fn register_builtin_plugins(&mut self) {
        self.register_plugin(Box::new(CrabEssentialsPlugin));
    }

    pub fn register_plugin(&mut self, plugin: Box<dyn CrabPlugin>) {
        info!(
            "Loading plugin {} v{}...",
            plugin.name(),
            plugin.version()
        );
        self.plugins.push(plugin);
    }

    pub fn enable_all(&mut self, server_version: &str) {
        info!("Enabling {} loaded plugin(s)...", self.plugins.len());
        for plugin in &mut self.plugins {
            let data_folder = self.plugins_dir.join(plugin.name());
            let _ = fs::create_dir_all(&data_folder);

            let ctx = PluginContext {
                server_version: server_version.to_string(),
                data_folder,
            };

            if let Err(e) = plugin.on_enable(&ctx) {
                error!("Error enabling plugin {}: {}", plugin.name(), e);
            }
        }
    }

    pub fn disable_all(&mut self) {
        info!("Disabling plugins...");
        for plugin in &mut self.plugins {
            if let Err(e) = plugin.on_disable() {
                error!("Error disabling plugin {}: {}", plugin.name(), e);
            }
        }
    }

    pub fn dispatch_command(&mut self, sender: &str, command: &str, args: &[&str]) -> bool {
        for plugin in &mut self.plugins {
            if plugin.on_command(sender, command, args) {
                return true;
            }
        }
        false
    }

    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins
            .iter()
            .map(|p| format!("{} (v{})", p.name(), p.version()))
            .collect()
    }
}
