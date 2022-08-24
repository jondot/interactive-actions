//!
//! `interactive-actions` is a library for automating running scripts and human interactions in a declarative way.
//!
//! ## Actions
//! This crate uses a set of [`Action`]s to describe a workflow. You can give it a
//! `name`, custom script to run with `run`, and an [`Interaction`][data::Interaction] for interacting against
//! a human.
//!
//! You also have additional control flags such as `ignore_exit`, `capture` and others. See below:
//!
//!
//! ## Examples
//! Run a script conditionally, only after confirming:
//!
//! ```no_run
//! use interactive_actions::data::{Action, ActionHook, VarBag};
//! use interactive_actions::ActionRunner;
//! use std::path::Path;
//!
//! let actions_defs: Vec<Action> = serde_yaml::from_str(
//! r#"
//! - name: confirm-action
//!   interaction:
//!     kind: confirm
//!     prompt: are you sure?
//!   run: echo hello
//! "#).unwrap();
//!
//! let mut actions = ActionRunner::default();
//! let mut v = VarBag::new();
//! // give it a current working folder `.` and a progress function
//! actions.run(&actions_defs, Some(Path::new(".")), &mut v, ActionHook::After, Some(|action: &Action| { println!("running: {:?}", action) }));
//!```
//!
//! Describe a set of actions and interactive prompting, optionally using input variable capture,
//! and then run everything interatively.
//!
//!
//! ```no_run
//! use interactive_actions::data::{Action, ActionHook, VarBag};
//! use interactive_actions::ActionRunner;
//! use std::path::Path;
//!
//! let actions_defs: Vec<Action> = serde_yaml::from_str(
//! r#"
//! - name: confirm-action
//!   interaction:
//!     kind: confirm
//!     prompt: are you sure?
//!     out: confirm
//! - name: input-action
//!   interaction:
//!     kind: input
//!     prompt: which city?
//!     default: dallas
//!     out: city
//! - name: select-action
//!   interaction:
//!     kind: select
//!     prompt: select transport
//!     options:
//!     - bus
//!     - train
//!     - walk
//!     default: bus
//!     out: transport
//!   run: echo {{city}} {{transport}}
//! "#).unwrap();
//!
//! let mut actions = ActionRunner::default();
//! let mut v = VarBag::new();
//! actions.run(&actions_defs, Some(Path::new(".")), &mut v, ActionHook::After, None::<fn(&Action) -> ()>);
//!```
//!
//!
#![warn(missing_docs)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::use_self)]
#![allow(clippy::missing_const_for_fn)]

pub mod data;

use anyhow::{Error, Result};
use data::{Action, ActionHook, ActionResult, Response, RunResult, VarBag};
use requestty_ui::events::{KeyEvent, TestEvents};
use run_script::IoOptions;
use std::path::Path;
use std::vec::IntoIter;

///
/// Runs [`Action`]s and keeps track of variables in `varbag`.
///
#[derive(Default)]
pub struct ActionRunner {
    /// synthetic events to be injected to prompts, useful in tests
    pub events: Option<TestEvents<IntoIter<KeyEvent>>>,
}

impl ActionRunner {
    /// create with actions. does not run them yet.
    /// create with actions and a set of synthetic events for testing
    pub fn with_events(events: Vec<KeyEvent>) -> Self {
        Self {
            events: Some(TestEvents::new(events)),
        }
    }

    /// Runs actions
    ///
    /// # Errors
    ///
    /// This function will return an error when actions fail
    #[allow(clippy::needless_pass_by_value)]
    pub fn run<P>(
        &mut self,
        actions: &[Action],
        working_dir: Option<&Path>,
        varbag: &mut VarBag,
        hook: ActionHook,
        progress: Option<P>,
    ) -> Result<Vec<ActionResult>>
    where
        P: Fn(&Action),
    {
        actions
            .iter()
            .filter(|action| action.hook == hook)
            .map(|action| {
                // get interactive response from the user if any is defined
                if let Some(ref progress) = progress {
                    progress(action);
                }

                let response = action
                    .interaction
                    .as_ref()
                    .map_or(Ok(Response::None), |interaction| {
                        interaction.play(Some(varbag), self.events.as_mut())
                    });

                // with the defined run script and user response, perform an action
                response.and_then(|r| match (r, action.run.as_ref()) {
                    (Response::Cancel, _) => {
                        if action.break_if_cancel {
                            Err(anyhow::anyhow!("stop requested (break_if_cancel)"))
                        } else {
                            Ok(ActionResult {
                                name: action.name.clone(),
                                run: None,
                                response: Response::Cancel,
                            })
                        }
                    }
                    (resp, None) => Ok(ActionResult {
                        name: action.name.clone(),
                        run: None,
                        response: resp,
                    }),
                    (resp, Some(run)) => {
                        let mut options = run_script::ScriptOptions::new();
                        options.working_directory = working_dir.map(std::path::Path::to_path_buf);
                        options.output_redirection = if action.capture {
                            IoOptions::Pipe
                        } else {
                            IoOptions::Inherit
                        };
                        options.print_commands = true;
                        let args = vec![];

                        // varbag replacements: {{interaction.outvar}} -> value
                        let script = varbag.iter().fold(run.clone(), |acc, (k, v)| {
                            acc.replace(&format!("{{{{{}}}}}", k), v)
                        });

                        run_script::run(script.as_str(), &args, &options)
                            .map_err(Error::msg)
                            .and_then(|tup| {
                                if !action.ignore_exit && tup.0 != 0 {
                                    anyhow::bail!(
                                        "in action '{}': command returned exit code '{}'",
                                        action.name,
                                        tup.0
                                    )
                                }
                                Ok(tup)
                            })
                            .map(|(code, out, err)| ActionResult {
                                name: action.name.clone(),
                                run: Some(RunResult {
                                    script,
                                    code,
                                    out,
                                    err,
                                }),
                                response: resp,
                            })
                    }
                })
            })
            .collect::<Result<Vec<_>>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use requestty_ui::events::KeyCode;

    #[test]
    fn test_interaction() {
        let actions_defs: Vec<Action> = serde_yaml::from_str(
            r#"
- name: confirm-action
  interaction:
    kind: confirm
    prompt: are you sure?
    out: confirm
- name: input-action
  interaction:
    kind: input
    prompt: which city?
    default: dallas
    out: city
- name: select-action
  interaction:
    kind: select
    prompt: select transport
    options:
    - bus
    - train
    - walk
    default: bus
"#,
        )
        .unwrap();
        let events = vec![
            KeyCode::Char('y').into(), // confirm: y
            KeyCode::Enter.into(),     //
            KeyCode::Char('t').into(), // city: 'tlv'
            KeyCode::Char('l').into(), //
            KeyCode::Char('v').into(), //
            KeyCode::Enter.into(),     //
            KeyCode::Down.into(),      // select: train
            KeyCode::Enter.into(),     //
        ];
        let mut actions = ActionRunner::with_events(events);
        let mut v = VarBag::new();
        assert_debug_snapshot!(actions
            .run(
                &actions_defs,
                Some(Path::new(".")),
                &mut v,
                ActionHook::After,
                None::<&fn(&Action) -> ()>
            )
            .unwrap());
        assert_debug_snapshot!(v);
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_run_script() {
        let actions_defs: Vec<Action> = serde_yaml::from_str(
            r#"
    - name: input-action
      interaction:
        kind: input
        prompt: which city?
        default: dallas
        out: city
      run: echo {{city}}
      capture: true
    "#,
        )
        .unwrap();
        let events = vec![
            KeyCode::Char('t').into(), // city: 'tlv'
            KeyCode::Char('l').into(), //
            KeyCode::Char('v').into(), //
            KeyCode::Enter.into(),     //
        ];
        let mut actions = ActionRunner::with_events(events);
        let mut v = VarBag::new();

        insta::assert_yaml_snapshot!(actions
            .run(
                &actions_defs,
                Some(Path::new(".")),
                &mut v,
                ActionHook::After,
            None::<&fn(&Action) -> ()>)
            .unwrap(),  {
            "[0].run.err" => ""
        });

        assert_debug_snapshot!(v);
    }
}
