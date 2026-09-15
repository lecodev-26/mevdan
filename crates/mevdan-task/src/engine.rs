//! `TaskEngine` — gestión de tareas y dependencias.

use crate::{
    error::{TaskError, TaskResult},
    id::TaskId,
    status::TaskStatus,
    task::Task,
};
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashSet, VecDeque};

/// Gestor de tareas.
#[derive(Debug, Clone, Default)]
pub struct TaskEngine {
    tasks: BTreeMap<TaskId, Task>,
}

impl TaskEngine {
    /// Crea un engine vacío.
    pub fn new() -> Self {
        Self::default()
    }

    // ──────────────────────────────────────────────
    // CRUD
    // ──────────────────────────────────────────────

    /// Añade una tarea.
    pub fn add(&mut self, task: Task) -> TaskResult<TaskId> {
        if task.is_blank() {
            return Err(TaskError::EmptyTitle(task.id.to_string()));
        }
        let id = task.id;
        if self.tasks.contains_key(&id) {
            return Err(TaskError::DuplicateTask(id.to_string()));
        }
        self.tasks.insert(id, task);
        Ok(id)
    }

    /// Elimina una tarea. Falla si otras tareas dependen de ella.
    pub fn remove(&mut self, id: TaskId) -> TaskResult<Task> {
        let dependents = self.dependents_of(id);
        if !dependents.is_empty() {
            return Err(TaskError::Blocked(format!(
                "task {} has dependents: {:?}",
                id, dependents
            )));
        }
        self.tasks
            .remove(&id)
            .ok_or_else(|| TaskError::TaskNotFound(id.to_string()))
    }

    pub fn get(&self, id: TaskId) -> Option<&Task> {
        self.tasks.get(&id)
    }

    pub fn get_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.get_mut(&id)
    }

    pub fn require(&self, id: TaskId) -> TaskResult<&Task> {
        self.get(id)
            .ok_or_else(|| TaskError::TaskNotFound(id.to_string()))
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn list(&self) -> Vec<&Task> {
        self.tasks.values().collect()
    }

    pub fn list_by_status(&self, status: TaskStatus) -> Vec<&Task> {
        self.tasks.values().filter(|t| t.status == status).collect()
    }

    // ──────────────────────────────────────────────
    // Dependencias
    // ──────────────────────────────────────────────

    /// Añade una dependencia: `task` depende de `depends_on`.
    pub fn add_dependency(&mut self, task: TaskId, depends_on: TaskId) -> TaskResult<()> {
        if task == depends_on {
            return Err(TaskError::SelfDependency {
                task: task.to_string(),
            });
        }
        if !self.tasks.contains_key(&task) {
            return Err(TaskError::TaskNotFound(task.to_string()));
        }
        if !self.tasks.contains_key(&depends_on) {
            return Err(TaskError::DependencyNotFound(depends_on.to_string()));
        }
        if self.would_create_cycle(task, depends_on) {
            return Err(TaskError::CycleDetected(format!(
                "{} → {}",
                task, depends_on
            )));
        }

        let t = self.tasks.get_mut(&task).unwrap();
        if !t.depends_on.contains(&depends_on) {
            t.depends_on.push(depends_on);
            t.touch();
        }
        Ok(())
    }

    /// ¿Añadir `task → depends_on` crearía un ciclo?
    fn would_create_cycle(&self, task: TaskId, depends_on: TaskId) -> bool {
        let mut visited: HashSet<TaskId> = HashSet::new();
        let mut queue: VecDeque<TaskId> = VecDeque::new();
        queue.push_back(depends_on);
        visited.insert(depends_on);

        while let Some(current) = queue.pop_front() {
            if current == task {
                return true;
            }
            if let Some(t) = self.tasks.get(&current) {
                for dep in &t.depends_on {
                    if visited.insert(*dep) {
                        queue.push_back(*dep);
                    }
                }
            }
        }
        false
    }

    pub fn dependencies_of(&self, id: TaskId) -> Vec<TaskId> {
        self.tasks
            .get(&id)
            .map(|t| t.depends_on.clone())
            .unwrap_or_default()
    }

    pub fn dependents_of(&self, id: TaskId) -> Vec<TaskId> {
        self.tasks
            .values()
            .filter(|t| t.depends_on.contains(&id))
            .map(|t| t.id)
            .collect()
    }

    /// ¿Todas las dependencias de `id` están completadas?
    pub fn dependencies_resolved(&self, id: TaskId) -> bool {
        let t = match self.tasks.get(&id) {
            Some(t) => t,
            None => return false,
        };
        t.depends_on.iter().all(|dep| {
            self.tasks
                .get(dep)
                .map(|d| d.status == TaskStatus::Completed)
                .unwrap_or(false)
        })
    }

    // ──────────────────────────────────────────────
    // Transiciones
    // ──────────────────────────────────────────────

    pub fn transition(&mut self, id: TaskId, next: TaskStatus) -> TaskResult<()> {
        if next == TaskStatus::Running && !self.dependencies_resolved(id) {
            return Err(TaskError::Blocked(id.to_string()));
        }

        let task = self
            .tasks
            .get_mut(&id)
            .ok_or_else(|| TaskError::TaskNotFound(id.to_string()))?;
        task.transition_to(next)
    }

    /// Recalcula el estado de las tareas `Pending`/`Blocked`.
    pub fn refresh_ready_tasks(&mut self) -> Vec<TaskId> {
        let mut updated = Vec::new();
        let ids: Vec<TaskId> = self.tasks.keys().copied().collect();

        for id in ids {
            let status = match self.tasks.get(&id) {
                Some(t) => t.status,
                None => continue,
            };
            if !matches!(status, TaskStatus::Pending | TaskStatus::Blocked) {
                continue;
            }
            if self.dependencies_resolved(id) {
                if let Some(t) = self.tasks.get_mut(&id) {
                    if t.transition_to(TaskStatus::Ready).is_ok() {
                        updated.push(id);
                    }
                }
            }
        }
        updated
    }

    /// Tareas listas para ejecutar (`Ready`), ordenadas por prioridad
    /// descendente (mayor prioridad primero).
    pub fn ready_tasks(&self) -> Vec<&Task> {
        let mut ready: Vec<&Task> = self
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::Ready)
            .collect();
        // `Reverse` invierte el orden: mayor prioridad primero.
        ready.sort_by_key(|t| Reverse(t.priority));
        ready
    }

    // ──────────────────────────────────────────────
    // Consultas
    // ──────────────────────────────────────────────

    pub fn all_terminal(&self) -> bool {
        self.tasks.values().all(|t| t.is_terminal())
    }

    pub fn has_failures(&self) -> bool {
        self.tasks.values().any(|t| t.status == TaskStatus::Failed)
    }

    /// Orden topológico de tareas (por dependencias).
    pub fn topological_order(&self) -> Option<Vec<TaskId>> {
        // Kahn's algorithm.
        let mut in_degree: BTreeMap<TaskId, usize> = BTreeMap::new();
        for id in self.tasks.keys() {
            in_degree.insert(*id, 0);
        }

        // in_degree[t] = número de dependencias de t.
        for t in self.tasks.values() {
            let count = t.depends_on.len();
            if let Some(d) = in_degree.get_mut(&t.id) {
                *d = count;
            }
        }

        let mut queue: VecDeque<TaskId> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut result: Vec<TaskId> = Vec::with_capacity(self.tasks.len());
        while let Some(id) = queue.pop_front() {
            result.push(id);
            for t in self.tasks.values() {
                if t.depends_on.contains(&id) {
                    if let Some(d) = in_degree.get_mut(&t.id) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            queue.push_back(t.id);
                        }
                    }
                }
            }
        }

        if result.len() == self.tasks.len() {
            Some(result)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::TaskPriority;

    fn engine_with_tasks() -> (TaskEngine, TaskId, TaskId) {
        let mut engine = TaskEngine::new();
        let t1 = engine.add(Task::new("first")).unwrap();
        let t2 = engine.add(Task::new("second")).unwrap();
        (engine, t1, t2)
    }

    #[test]
    fn new_engine_is_empty() {
        let e = TaskEngine::new();
        assert!(e.is_empty());
        assert_eq!(e.len(), 0);
    }

    #[test]
    fn add_task_returns_id() {
        let mut e = TaskEngine::new();
        let id = e.add(Task::new("t")).unwrap();
        assert!(e.get(id).is_some());
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn add_blank_task_fails() {
        let mut e = TaskEngine::new();
        let err = e.add(Task::new("")).unwrap_err();
        assert!(matches!(err, TaskError::EmptyTitle(_)));
    }

    #[test]
    fn add_duplicate_id_fails() {
        let mut e = TaskEngine::new();
        let t = Task::new("t");
        let id = t.id;
        e.add(t).unwrap();

        let dup = Task {
            id,
            ..Task::new("other")
        };
        let err = e.add(dup).unwrap_err();
        assert!(matches!(err, TaskError::DuplicateTask(_)));
    }

    #[test]
    fn remove_task_works() {
        let (mut engine, t1, _t2) = engine_with_tasks();
        engine.remove(t1).unwrap();
        assert_eq!(engine.len(), 1);
    }

    #[test]
    fn remove_unknown_fails() {
        let mut e = TaskEngine::new();
        let err = e.remove(TaskId::new()).unwrap_err();
        assert!(matches!(err, TaskError::TaskNotFound(_)));
    }

    #[test]
    fn remove_task_with_dependents_fails() {
        let (mut engine, t1, t2) = engine_with_tasks();
        engine.add_dependency(t2, t1).unwrap();
        let err = engine.remove(t1).unwrap_err();
        assert!(matches!(err, TaskError::Blocked(_)));
    }

    #[test]
    fn add_dependency_works() {
        let (mut engine, t1, t2) = engine_with_tasks();
        engine.add_dependency(t2, t1).unwrap();
        assert_eq!(engine.dependencies_of(t2), vec![t1]);
        assert_eq!(engine.dependents_of(t1), vec![t2]);
    }

    #[test]
    fn self_dependency_fails() {
        let (mut engine, t1, _t2) = engine_with_tasks();
        let err = engine.add_dependency(t1, t1).unwrap_err();
        assert!(matches!(err, TaskError::SelfDependency { .. }));
    }

    #[test]
    fn dependency_to_unknown_fails() {
        let (mut engine, t1, _t2) = engine_with_tasks();
        let unknown = TaskId::new();
        let err = engine.add_dependency(t1, unknown).unwrap_err();
        assert!(matches!(err, TaskError::DependencyNotFound(_)));
    }

    #[test]
    fn dependency_from_unknown_fails() {
        let (mut engine, _t1, t2) = engine_with_tasks();
        let unknown = TaskId::new();
        let err = engine.add_dependency(unknown, t2).unwrap_err();
        assert!(matches!(err, TaskError::TaskNotFound(_)));
    }

    #[test]
    fn cycle_detection() {
        let mut e = TaskEngine::new();
        let t1 = e.add(Task::new("a")).unwrap();
        let t2 = e.add(Task::new("b")).unwrap();
        let t3 = e.add(Task::new("c")).unwrap();

        e.add_dependency(t2, t1).unwrap();
        e.add_dependency(t3, t2).unwrap();
        let err = e.add_dependency(t1, t3).unwrap_err();
        assert!(matches!(err, TaskError::CycleDetected(_)));
    }

    #[test]
    fn duplicate_dependency_is_idempotent() {
        let (mut engine, t1, t2) = engine_with_tasks();
        engine.add_dependency(t2, t1).unwrap();
        engine.add_dependency(t2, t1).unwrap();
        assert_eq!(engine.dependencies_of(t2).len(), 1);
    }

    #[test]
    fn dependencies_resolved_when_dep_completed() {
        let (mut engine, t1, t2) = engine_with_tasks();
        engine.add_dependency(t2, t1).unwrap();

        assert!(!engine.dependencies_resolved(t2));

        engine.transition(t1, TaskStatus::Ready).unwrap();
        engine.transition(t1, TaskStatus::Running).unwrap();
        engine.transition(t1, TaskStatus::Completed).unwrap();

        assert!(engine.dependencies_resolved(t2));
    }

    #[test]
    fn transition_valid() {
        let (mut engine, t1, _) = engine_with_tasks();
        engine.transition(t1, TaskStatus::Ready).unwrap();
        assert_eq!(engine.get(t1).unwrap().status, TaskStatus::Ready);
    }

    #[test]
    fn transition_to_running_blocked_by_deps() {
        let (mut engine, t1, t2) = engine_with_tasks();
        engine.add_dependency(t2, t1).unwrap();

        engine.transition(t2, TaskStatus::Ready).unwrap();
        let err = engine.transition(t2, TaskStatus::Running).unwrap_err();
        assert!(matches!(err, TaskError::Blocked(_)));
    }

    #[test]
    fn transition_to_running_after_deps_resolved() {
        let (mut engine, t1, t2) = engine_with_tasks();
        engine.add_dependency(t2, t1).unwrap();

        engine.transition(t1, TaskStatus::Ready).unwrap();
        engine.transition(t1, TaskStatus::Running).unwrap();
        engine.transition(t1, TaskStatus::Completed).unwrap();

        engine.transition(t2, TaskStatus::Ready).unwrap();
        engine.transition(t2, TaskStatus::Running).unwrap();
        assert_eq!(engine.get(t2).unwrap().status, TaskStatus::Running);
    }

    #[test]
    fn refresh_ready_tasks_promotes_pending() {
        let (mut engine, t1, t2) = engine_with_tasks();
        engine.add_dependency(t2, t1).unwrap();

        let updated = engine.refresh_ready_tasks();
        assert_eq!(updated, vec![t1]);
        assert_eq!(engine.get(t1).unwrap().status, TaskStatus::Ready);
        assert_eq!(engine.get(t2).unwrap().status, TaskStatus::Pending);
    }

    #[test]
    fn ready_tasks_sorted_by_priority() {
        let mut e = TaskEngine::new();
        let low = e
            .add(Task::new("low").with_priority(TaskPriority::Low))
            .unwrap();
        let high = e
            .add(Task::new("high").with_priority(TaskPriority::High))
            .unwrap();
        let crit = e
            .add(Task::new("critical").with_priority(TaskPriority::Critical))
            .unwrap();

        e.transition(low, TaskStatus::Ready).unwrap();
        e.transition(high, TaskStatus::Ready).unwrap();
        e.transition(crit, TaskStatus::Ready).unwrap();

        let ready = e.ready_tasks();
        assert_eq!(ready.len(), 3);
        assert_eq!(ready[0].id, crit);
        assert_eq!(ready[1].id, high);
        assert_eq!(ready[2].id, low);
    }

    #[test]
    fn all_terminal_works() {
        let (mut engine, t1, t2) = engine_with_tasks();
        assert!(!engine.all_terminal());

        for t in [t1, t2] {
            engine.transition(t, TaskStatus::Ready).unwrap();
            engine.transition(t, TaskStatus::Running).unwrap();
            engine.transition(t, TaskStatus::Completed).unwrap();
        }
        assert!(engine.all_terminal());
    }

    #[test]
    fn has_failures_works() {
        let (mut engine, t1, _) = engine_with_tasks();
        assert!(!engine.has_failures());

        engine.transition(t1, TaskStatus::Ready).unwrap();
        engine.transition(t1, TaskStatus::Running).unwrap();
        engine.transition(t1, TaskStatus::Failed).unwrap();

        assert!(engine.has_failures());
    }

    #[test]
    fn topological_order_linear() {
        let mut e = TaskEngine::new();
        let t1 = e.add(Task::new("a")).unwrap();
        let t2 = e.add(Task::new("b")).unwrap();
        let t3 = e.add(Task::new("c")).unwrap();
        e.add_dependency(t2, t1).unwrap();
        e.add_dependency(t3, t2).unwrap();

        let order = e.topological_order().unwrap();
        assert_eq!(order, vec![t1, t2, t3]);
    }

    #[test]
    fn list_by_status_works() {
        let (mut engine, t1, _t2) = engine_with_tasks();
        engine.transition(t1, TaskStatus::Ready).unwrap();

        assert_eq!(engine.list_by_status(TaskStatus::Ready).len(), 1);
        assert_eq!(engine.list_by_status(TaskStatus::Pending).len(), 1);
    }

    #[test]
    fn realistic_workflow() {
        let mut e = TaskEngine::new();

        let setup = e.add(Task::new("setup project")).unwrap();
        let write = e.add(Task::new("write code")).unwrap();
        let test = e.add(Task::new("run tests")).unwrap();
        let deploy = e.add(Task::new("deploy")).unwrap();

        e.add_dependency(write, setup).unwrap();
        e.add_dependency(test, write).unwrap();
        e.add_dependency(deploy, test).unwrap();

        let updated = e.refresh_ready_tasks();
        assert_eq!(updated, vec![setup]);

        e.transition(setup, TaskStatus::Running).unwrap();
        e.transition(setup, TaskStatus::Completed).unwrap();

        let updated = e.refresh_ready_tasks();
        assert_eq!(updated, vec![write]);

        e.transition(write, TaskStatus::Running).unwrap();
        e.transition(write, TaskStatus::Completed).unwrap();

        let updated = e.refresh_ready_tasks();
        assert_eq!(updated, vec![test]);

        e.transition(test, TaskStatus::Running).unwrap();
        e.transition(test, TaskStatus::Completed).unwrap();

        let updated = e.refresh_ready_tasks();
        assert_eq!(updated, vec![deploy]);

        e.transition(deploy, TaskStatus::Running).unwrap();
        e.transition(deploy, TaskStatus::Completed).unwrap();

        assert!(e.all_terminal());
        assert!(!e.has_failures());

        let order = e.topological_order().unwrap();
        assert_eq!(order, vec![setup, write, test, deploy]);
    }
}
