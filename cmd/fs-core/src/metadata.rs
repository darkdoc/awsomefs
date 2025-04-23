use std::collections::HashMap;
use crate::volume::Volume;

pub struct MetadataStore {
    volumes: HashMap<String, Volume>,
}

impl MetadataStore {
    pub fn new() -> Self {
        Self {
            volumes: HashMap::new(),
        }
    }

    pub fn create_volume(&mut self, id: String, size: u64, offset: u64) {
        let volume = Volume { id: id.clone(), size, offset };
        self.volumes.insert(id, volume);
    }

    pub fn get_volume(&self, id: &str) -> Option<&Volume> {
        self.volumes.get(id)
    }
}
