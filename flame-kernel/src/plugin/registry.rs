use crate::core::error::AppError;
use crate::domain::entity::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct PluginRegistry {
    plugins: Arc<Mutex<HashMap<String, Plugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register(&self, plugin: Plugin) -> Result<(), AppError> {
        let mut plugins = self.plugins.lock().unwrap();
        if plugins.contains_key(&plugin.id) {
            return Err(AppError::BadRequest(format!(
                "Plugin {} already registered",
                plugin.id
            )));
        }
        self.validate_dependencies(&plugin, &plugins)?;
        plugins.insert(plugin.id.clone(), plugin);
        Ok(())
    }

    pub fn unregister(&self, id: &str) -> Result<Plugin, AppError> {
        let mut plugins = self.plugins.lock().unwrap();
        let dept_plugins: Vec<String> = plugins
            .values()
            .filter(|p| p.dependencies.iter().any(|d| d.plugin_id == id))
            .map(|p| p.id.clone())
            .collect();
        if !dept_plugins.is_empty() {
            return Err(AppError::BadRequest(format!(
                "Cannot unregister '{}': depended on by {:?}",
                id, dept_plugins
            )));
        }
        plugins
            .remove(id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))
    }

    pub fn get(&self, id: &str) -> Result<Plugin, AppError> {
        let plugins = self.plugins.lock().unwrap();
        plugins
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))
    }

    pub fn list_all(&self) -> Vec<Plugin> {
        let plugins = self.plugins.lock().unwrap();
        plugins.values().cloned().collect()
    }

    pub fn enable(&self, id: &str) -> Result<Plugin, AppError> {
        let mut plugins = self.plugins.lock().unwrap();
        let plugin = plugins
            .get_mut(id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
        plugin.enabled = true;
        plugin.updated_at = chrono::Utc::now();
        Ok(plugin.clone())
    }

    pub fn disable(&self, id: &str) -> Result<Plugin, AppError> {
        let mut plugins = self.plugins.lock().unwrap();
        let plugin = plugins
            .get_mut(id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
        plugin.enabled = false;
        plugin.updated_at = chrono::Utc::now();
        Ok(plugin.clone())
    }

    pub fn exists(&self, id: &str) -> bool {
        let plugins = self.plugins.lock().unwrap();
        plugins.contains_key(id)
    }

    fn validate_dependencies(
        &self,
        plugin: &Plugin,
        existing: &HashMap<String, Plugin>,
    ) -> Result<(), AppError> {
        for dep in &plugin.dependencies {
            let dep_plugin = existing.get(&dep.plugin_id);
            match dep_plugin {
                Some(p) => {
                    if !p.enabled && !dep.optional {
                        return Err(AppError::BadRequest(format!(
                            "Dependency '{}' required by '{}' is disabled",
                            dep.plugin_id, plugin.id
                        )));
                    }
                }
                None => {
                    if !dep.optional {
                        return Err(AppError::BadRequest(format!(
                            "Missing required dependency '{}' for plugin '{}'",
                            dep.plugin_id, plugin.id
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<Plugin> {
        let plugins = self.plugins.lock().unwrap();
        plugins
            .values()
            .filter(|p| p.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<Plugin> {
        let q = query.to_lowercase();
        let plugins = self.plugins.lock().unwrap();
        plugins
            .values()
            .filter(|p| {
                p.id.to_lowercase().contains(&q)
                    || p.name.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
                    || p.author.to_lowercase().contains(&q)
                    || p.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .cloned()
            .collect()
    }
}
impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
