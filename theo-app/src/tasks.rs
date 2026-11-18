//! ⚠️ MOCK of custom tasks (Departments › Custom tasks): any task the congregation needs that
//! has no page of its own, with the people who do it. Scheduling comes with the other departments.

use crate::people::{fold, Directory};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CustomTask {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    /// People who do this task.
    pub people: Vec<u32>,
}

impl Directory {
    pub fn custom_task(&self, id: u32) -> Option<&CustomTask> {
        self.custom_tasks.iter().find(|t| t.id == id)
    }

    /// Adds (`id` = None) or changes a task. Names are unique.
    pub fn save_custom_task(&mut self, id: Option<u32>, name: &str, description: &str, people: Vec<u32>) -> Result<u32, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a name for the task.".into());
        }
        if self.custom_tasks.iter().any(|t| Some(t.id) != id && fold(&t.name) == fold(name)) {
            return Err("There is already a task with this name.".into());
        }
        let people: Vec<u32> = people.into_iter().filter(|p| self.person(*p).is_some_and(|x| x.moved_away.is_none())).collect();
        let description = Some(description.trim().to_string()).filter(|d| !d.is_empty());
        let new_id = id.unwrap_or_else(|| self.custom_tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1);
        let t = CustomTask { id: new_id, name: name.into(), description, people };
        match self.custom_tasks.iter_mut().find(|x| x.id == new_id) {
            Some(slot) => *slot = t,
            None => self.custom_tasks.push(t),
        }
        Ok(new_id)
    }

    pub fn delete_custom_task(&mut self, id: u32) {
        self.custom_tasks.retain(|t| t.id != id);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn task_names_are_required_and_unique() {
        let mut d = crate::mock::demo_directory(14);
        assert!(d.save_custom_task(None, " ", "", vec![]).is_err());
        let id = d.save_custom_task(None, "Window cleaning", "Twice a year", vec![1, 3, 999]).unwrap_or(0);
        assert_eq!(d.custom_task(id).map(|t| t.people.clone()), Some(vec![1, 3]));
        assert!(d.save_custom_task(None, "window CLEANING", "", vec![]).is_err());
    }
}
