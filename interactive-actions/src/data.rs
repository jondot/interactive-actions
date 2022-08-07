//!
//! doc for module
//!
use anyhow::Result;
use requestty::{Answer, Question};
use std::collections::BTreeMap;

use requestty_ui::backend::{Size, TestBackend};
use requestty_ui::events::{KeyEvent, TestEvents};
use serde_derive::{Deserialize, Serialize};
use std::vec::IntoIter;

#[doc(hidden)]
pub type VarBag = BTreeMap<String, String>;

///
/// [`Action`] defines the action to run:
/// * script
/// * interaction
/// * control flow and variable capture
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    /// unique name of action
    pub name: String,

    /// interaction
    #[serde(default)]
    pub interaction: Option<Interaction>,

    /// a run script
    pub run: Option<String>,

    /// ignore exit code from the script, otherwise if error then exists
    #[serde(default)]
    pub ignore_exit: bool,

    /// if confirm cancel, cancel all the rest of the actions and break out
    #[serde(default)]
    pub break_if_cancel: bool,

    /// captures the output of the script, otherwise, stream to screen in real time
    #[serde(default)]
    pub capture: bool,
}
///
/// result of the [`Action`]
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionResult {
    /// name of action that was run
    pub name: String,
    /// result of run script
    pub run: Option<RunResult>,
    /// interaction response, if any
    pub response: Response,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub script: String,
    pub code: i32,
    pub out: String,
    pub err: String,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InteractionKind {
    #[serde(rename = "confirm")]
    Confirm,
    #[serde(rename = "input")]
    Input,
    #[serde(rename = "select")]
    Select,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Text(String),
    Cancel,
    None,
}

///
/// [`Interaction`] models an interactive session with a user declaratively
/// You can pick from _confirm_, _input_, and other modes of prompting.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interaction {
    /// type of interaction
    pub kind: InteractionKind,
    /// what to ask the user
    pub prompt: String,
    /// if set, capture the value of answer, and set it to a variable name defined here
    pub out: Option<String>,

    /// define the set of options just for kind=select
    pub options: Option<Vec<String>>,
}
impl Interaction {
    fn update_varbag(&self, input: &str, varbag: Option<&mut VarBag>) {
        varbag.map(|bag| {
            self.out
                .as_ref()
                .map(|out| bag.insert(out.to_string(), input.to_string()))
        });
    }

    /// Play an interaction
    ///
    /// # Errors
    ///
    /// This function will return an error if text input failed
    pub fn play(
        &self,
        varbag: Option<&mut VarBag>,
        events: Option<&mut TestEvents<IntoIter<KeyEvent>>>,
    ) -> Result<Response> {
        let question = self.to_question();
        let answer = if let Some(events) = events {
            let mut backend = TestBackend::new(Size::from((50, 20)));
            requestty::prompt_one_with(question, &mut backend, events)
        } else {
            requestty::prompt_one(question)
        }?;

        Ok(match answer {
            Answer::String(input) => {
                self.update_varbag(&input, varbag);

                Response::Text(input)
            }
            Answer::ListItem(selected) => {
                self.update_varbag(&selected.text, varbag);
                Response::Text(selected.text)
            }
            Answer::Bool(confirmed) if confirmed => {
                let as_string = "true".to_string();
                self.update_varbag(&as_string, varbag);
                Response::Text(as_string)
            }
            _ => {
                Response::Cancel
                // not supported question types
            }
        })
    }

    /// Convert the interaction into a question
    pub fn to_question(&self) -> Question<'_> {
        match self.kind {
            InteractionKind::Input => Question::input("question")
                .message(self.prompt.clone())
                .build(),
            InteractionKind::Select => Question::select("question")
                .message(self.prompt.clone())
                .choices(self.options.clone().unwrap_or_default())
                .build(),
            InteractionKind::Confirm => Question::confirm("question")
                .message(self.prompt.clone())
                .build(),
        }
    }
}
