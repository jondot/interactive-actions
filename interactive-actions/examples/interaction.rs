use interactive_actions::{
    data::{Action, ActionHook, VarBag},
    ActionRunner,
};

const YAML: &str = r#"
- name: start
  interaction:
    kind: confirm
    prompt: "are you ready to start?"
  break_if_cancel: true
- name: city
  interaction:
    kind: input
    prompt: input a city
    out: city
- name: transport
  interaction:
    kind: select
    prompt: pick a transport
    options:
    - bus
    - train
    - bike
    default: bus
    out: transport
  run: echo go for {{city}} on a {{transport}}
"#;

fn main() {
    let actions: Vec<Action> = serde_yaml::from_str(YAML).unwrap();
    let mut runner = ActionRunner::default();
    let mut v = VarBag::new();
    let res = runner.run(
        &actions,
        None,
        &mut v,
        ActionHook::After,
        Some(|action: &Action| {
            println!("{}", action.name);
        }),
    );
    println!("{res:?}");
}
