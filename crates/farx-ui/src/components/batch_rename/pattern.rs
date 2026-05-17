use super::state::BatchRenameState;

impl BatchRenameState {
    pub(super) fn update_previews(&mut self) {
        if self.find_pattern.is_empty() {
            self.previews = self.files.iter().map(|(_, n)| n.clone()).collect();
            return;
        }
        match regex::Regex::new(&self.find_pattern) {
            Ok(re) => {
                self.previews = self
                    .files
                    .iter()
                    .map(|(_, name)| {
                        re.replace_all(name, self.replace_pattern.as_str())
                            .to_string()
                    })
                    .collect();
            }
            Err(_) => {
                // Invalid regex — fallback to literal replace
                self.previews = self
                    .files
                    .iter()
                    .map(|(_, name)| name.replace(&self.find_pattern, &self.replace_pattern))
                    .collect();
            }
        }
    }
}
