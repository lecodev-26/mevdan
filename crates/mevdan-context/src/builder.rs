//! Constructor de contexto.
//!
//! Toma secciones y las ordena/recorta según el presupuesto de tokens
//! y las prioridades. El resultado es un `Context` listo para
//! convertir a mensajes del provider.

use crate::{
    budget::{estimate_tokens, TokenBudget},
    error::{ContextError, ContextResult},
    section::{Priority, Section},
};
use mevdan_provider::Message;
use serde::{Deserialize, Serialize};

/// Contexto final, listo para enviar al provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Secciones incluidas (ya ordenadas por prioridad).
    pub sections: Vec<Section>,
    /// Tokens estimados del contexto completo.
    pub estimated_tokens: u32,
    /// Presupuesto usado.
    pub budget: TokenBudget,
}

impl Context {
    /// Convierte el contexto a una lista de mensajes para el provider.
    ///
    /// Cada sección se emite como un `Message::user` precedido por su
    /// título entre corchetes.
    pub fn to_messages(&self) -> Vec<Message> {
        self.sections
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| Message::user(format!("[{}]\n{}", s.title, s.content)))
            .collect()
    }

    /// Número de secciones.
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    /// ¿Cabe todo en el presupuesto?
    pub fn fits(&self) -> bool {
        self.estimated_tokens <= self.budget.available()
    }
}

/// Constructor de contexto.
#[derive(Debug)]
pub struct ContextBuilder {
    budget: TokenBudget,
    sections: Vec<Section>,
}

impl ContextBuilder {
    pub fn new(budget: TokenBudget) -> Self {
        Self {
            budget,
            sections: Vec::new(),
        }
    }

    /// Añade una sección genérica.
    ///
    /// Nota: el método se llama `with_section` (no `add`) para evitar
    /// la ambigüedad con `std::ops::Add::add` que detecta Clippy.
    pub fn with_section(mut self, section: Section) -> Self {
        self.sections.push(section);
        self
    }

    /// Añade el system prompt. Nunca se recorta.
    pub fn system(self, prompt: impl Into<String>) -> Self {
        self.with_section(Section::critical("System", prompt))
    }

    /// Añade la tarea del usuario.
    pub fn task(self, task: impl Into<String>) -> Self {
        self.with_section(Section::high("Task", task))
    }

    /// Añade contexto del proyecto.
    pub fn project(self, info: impl Into<String>) -> Self {
        self.with_section(Section::medium("Project", info))
    }

    /// Añade historial de conversación.
    pub fn history(self, history: impl Into<String>) -> Self {
        self.with_section(Section::low("History", history))
    }

    /// Añade referencias a archivos.
    pub fn files(self, files: impl Into<String>) -> Self {
        self.with_section(Section::optional("Files", files))
    }

    /// Construye el contexto.
    ///
    /// Reglas:
    /// 1. Ordena las secciones por prioridad (Critical → Optional).
    /// 2. Suma tokens estimados.
    /// 3. Si no cabe, recorta secciones truncatables por prioridad
    ///    ascendente (primero las de menor prioridad).
    /// 4. Si aun así no cabe y hay secciones críticas, error.
    pub fn build(self) -> ContextResult<Context> {
        // 1. Verificar que el system prompt existe y no está vacío.
        let has_system = self
            .sections
            .iter()
            .any(|s| s.priority == Priority::Critical && !s.is_empty());
        if !has_system {
            return Err(ContextError::EmptyRequired("system prompt".into()));
        }

        // 2. Ordenar por prioridad.
        let mut sections = self.sections;
        sections.sort_by_key(|s| s.priority);

        // 3. Comprobar si cabe tal cual.
        let available = self.budget.available();
        let total = total_tokens(&sections);

        if total <= available {
            return Ok(Context {
                sections,
                estimated_tokens: total,
                budget: self.budget,
            });
        }

        // 4. Recortar. Iteramos sobre los índices (no sobre las
        //    secciones) para poder recalcular el total sin pelearse
        //    con el borrow checker.
        let truncate_order = truncation_order(&sections);
        for idx in truncate_order {
            if total_tokens(&sections) <= available {
                break;
            }
            // Trunca a la mitad y recalcula.
            let half = sections[idx].content.len() / 2;
            sections[idx].truncate_to(half);
            if total_tokens(&sections) > available {
                sections[idx].content.clear();
            }
        }

        // 5. Verificación final.
        let final_total = total_tokens(&sections);
        if final_total > available {
            return Err(ContextError::BudgetExceeded {
                used: final_total,
                max: available,
            });
        }

        Ok(Context {
            sections,
            estimated_tokens: final_total,
            budget: self.budget,
        })
    }
}

/// Calcula el orden de truncado: índice de las secciones truncatables,
/// de menor prioridad a mayor.
fn truncation_order(sections: &[Section]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..sections.len())
        .filter(|&i| sections[i].truncatable)
        .collect();
    // Ordenamos por prioridad descendente (menor prioridad primero).
    indices.sort_by_key(|&i| std::cmp::Reverse(sections[i].priority));
    indices
}

fn total_tokens(sections: &[Section]) -> u32 {
    sections.iter().map(|s| estimate_tokens(&s.content)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_requires_system() {
        let builder = ContextBuilder::new(TokenBudget::default_budget()).task("do something");
        let err = builder.build().unwrap_err();
        assert!(matches!(err, ContextError::EmptyRequired(_)));
    }

    #[test]
    fn build_requires_non_empty_system() {
        let builder = ContextBuilder::new(TokenBudget::default_budget())
            .system("   ")
            .task("do something");
        let err = builder.build().unwrap_err();
        assert!(matches!(err, ContextError::EmptyRequired(_)));
    }

    #[test]
    fn build_minimal_context() {
        let context = ContextBuilder::new(TokenBudget::default_budget())
            .system("you are helpful")
            .task("say hi")
            .build()
            .unwrap();

        assert_eq!(context.section_count(), 2);
        assert!(context.fits());
    }

    #[test]
    fn build_orders_by_priority() {
        let context = ContextBuilder::new(TokenBudget::default_budget())
            .system("sys")
            .task("task")
            .project("project info")
            .history("history")
            .files("files")
            .build()
            .unwrap();

        let priorities: Vec<Priority> = context.sections.iter().map(|s| s.priority).collect();
        assert_eq!(
            priorities,
            vec![
                Priority::Critical,
                Priority::High,
                Priority::Medium,
                Priority::Low,
                Priority::Optional,
            ]
        );
    }

    #[test]
    fn build_truncates_when_over_budget() {
        // Presupuesto pequeño pero con historial gigante.
        let budget = TokenBudget {
            max_tokens: 200,
            reserve_for_response: 50,
        };
        let big_history = "x".repeat(4000);

        let context = ContextBuilder::new(budget)
            .system("sys")
            .task("task")
            .history(big_history)
            .build()
            .unwrap();

        // El historial debe haberse recortado o vaciado.
        let history = context
            .sections
            .iter()
            .find(|s| s.priority == Priority::Low)
            .unwrap();
        assert!(history.byte_len() < 4000);
    }

    #[test]
    fn build_fails_when_system_alone_overflows() {
        let budget = TokenBudget {
            max_tokens: 10,
            reserve_for_response: 5,
        };
        let huge_system = "x".repeat(10_000);

        let err = ContextBuilder::new(budget)
            .system(huge_system)
            .task("task")
            .build()
            .unwrap_err();

        assert!(matches!(err, ContextError::BudgetExceeded { .. }));
    }

    #[test]
    fn to_messages_includes_titles() {
        let context = ContextBuilder::new(TokenBudget::default_budget())
            .system("sys")
            .task("do X")
            .build()
            .unwrap();

        let messages = context.to_messages();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].content.contains("[System]"));
        assert!(messages[0].content.contains("sys"));
        assert!(messages[1].content.contains("[Task]"));
        assert!(messages[1].content.contains("do X"));
    }

    #[test]
    fn to_messages_skips_empty() {
        let context = ContextBuilder::new(TokenBudget::default_budget())
            .system("sys")
            .task("task")
            .history("")
            .build()
            .unwrap();

        let messages = context.to_messages();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn context_fits_with_small_content() {
        let context = ContextBuilder::new(TokenBudget::default_budget())
            .system("sys")
            .task("small task")
            .build()
            .unwrap();
        assert!(context.fits());
    }

    #[test]
    fn context_roundtrips() {
        let context = ContextBuilder::new(TokenBudget::default_budget())
            .system("sys")
            .task("task")
            .build()
            .unwrap();

        let json = serde_json::to_string(&context).unwrap();
        let back: Context = serde_json::from_str(&json).unwrap();
        assert_eq!(back.section_count(), context.section_count());
        assert_eq!(back.estimated_tokens, context.estimated_tokens);
    }

    #[test]
    fn truncation_prefers_lower_priority_sections() {
        // Presupuesto suficiente para system+task, pero no para el
        // historial gigante.
        let budget = TokenBudget {
            max_tokens: 500,
            reserve_for_response: 100,
        };
        let big_history = "x".repeat(3000);
        let big_files = "y".repeat(3000);

        let context = ContextBuilder::new(budget)
            .system("short system prompt")
            .task("do something")
            .history(big_history)
            .files(big_files)
            .build()
            .unwrap();

        // System y task deben estar intactos.
        let system = context
            .sections
            .iter()
            .find(|s| s.priority == Priority::Critical)
            .unwrap();
        assert_eq!(system.content, "short system prompt");

        let task = context
            .sections
            .iter()
            .find(|s| s.priority == Priority::High)
            .unwrap();
        assert_eq!(task.content, "do something");

        // History y files deben haberse truncado.
        let history = context
            .sections
            .iter()
            .find(|s| s.priority == Priority::Low)
            .unwrap();
        let files = context
            .sections
            .iter()
            .find(|s| s.priority == Priority::Optional)
            .unwrap();
        assert!(history.byte_len() < 3000);
        assert!(files.byte_len() < 3000);
    }

    #[test]
    fn with_section_adds_custom_section() {
        let context = ContextBuilder::new(TokenBudget::default_budget())
            .system("sys")
            .with_section(Section::high("Custom", "custom content"))
            .build()
            .unwrap();

        assert_eq!(context.section_count(), 2);
        assert!(context.sections.iter().any(|s| s.title == "Custom"));
    }
}
