use mlua::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::{lua_err, plugin_directory, PluginCommand, PluginResult};

/// The plugin engine manages Lua state and registered commands.
pub struct PluginEngine {
    lua: Lua,
    commands: HashMap<String, PluginCommand>,
    plugin_dir: PathBuf,
}

impl PluginEngine {
    pub fn new() -> anyhow::Result<Self> {
        let lua = Lua::new();
        Ok(Self {
            lua,
            commands: HashMap::new(),
            plugin_dir: plugin_directory(),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_plugin_dir(plugin_dir: PathBuf) -> anyhow::Result<Self> {
        let lua = Lua::new();
        Ok(Self {
            lua,
            commands: HashMap::new(),
            plugin_dir,
        })
    }

    /// Load all plugins from the plugins directory.
    pub fn load_plugins(&mut self) -> anyhow::Result<Vec<String>> {
        let plugin_dir = self.plugin_dir.clone();
        let mut loaded = Vec::new();

        if !plugin_dir.exists() {
            return Ok(loaded);
        }

        let entries = std::fs::read_dir(&plugin_dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("lua") {
                let name = path
                    .file_stem()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                match self.load_plugin(&path) {
                    Ok(cmds) => {
                        for cmd in cmds {
                            self.commands.insert(cmd.name.clone(), cmd);
                        }
                        loaded.push(name);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load plugin {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(loaded)
    }

    fn load_plugin(&self, path: &PathBuf) -> anyhow::Result<Vec<PluginCommand>> {
        let source = std::fs::read_to_string(path)?;
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Wrapper that captures register_command calls into a Lua table
        let wrapper = format!(
            r#"
local _cmds = {{}}
farx = farx or {{}}
farx.register_command = function(name, desc, body)
    table.insert(_cmds, {{name=name, desc=desc, body=body}})
end
{}
return _cmds
"#,
            source
        );

        let result: LuaTable = self.lua.load(&wrapper).eval().map_err(lua_err)?;

        let mut commands = Vec::new();
        for pair in result.sequence_values::<LuaTable>() {
            let table = pair.map_err(lua_err)?;
            let name: String = table.get("name").map_err(lua_err)?;
            let desc: String = table.get("desc").map_err(lua_err)?;
            commands.push(PluginCommand {
                name,
                description: desc,
                plugin_file: file_name.clone(),
            });
        }

        Ok(commands)
    }

    /// Execute a plugin command by name.
    pub fn execute_command(&self, name: &str, current_dir: &str) -> anyhow::Result<PluginResult> {
        let cmd = self
            .commands
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown plugin command: {}", name))?;

        let source = std::fs::read_to_string(self.plugin_dir.join(&cmd.plugin_file))?;

        let wrapper = format!(
            r#"
_farx_output = nil
farx = farx or {{}}
farx.current_dir = "{dir}"
farx.message = function(msg)
    _farx_output = msg
end
local _target_body = nil
farx.register_command = function(name, desc, body)
    if name == "{cmd_name}" then
        _target_body = body
    end
end
{source}
if _target_body then
    local fn_code = load(_target_body)
    if fn_code then fn_code() end
end
return _farx_output
"#,
            cmd_name = name,
            dir = current_dir.replace('\\', "\\\\").replace('"', "\\\""),
            source = source,
        );

        let result: Option<String> = self.lua.load(&wrapper).eval().map_err(lua_err)?;

        match result {
            Some(msg) => Ok(PluginResult::Message(msg)),
            None => Ok(PluginResult::None),
        }
    }

    pub fn list_commands(&self) -> Vec<&PluginCommand> {
        let mut cmds: Vec<&PluginCommand> = self.commands.values().collect();
        cmds.sort_by(|a, b| a.name.cmp(&b.name));
        cmds
    }

    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }
}
