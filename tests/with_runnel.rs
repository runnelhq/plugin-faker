//! The plugin in Runnel itself (RUNNEL_BIN): a pipeline faker.rows → core.to-result,
//! started by a workflow, and the check with init.

use runnel_plugin::testing::Project;

#[test]
fn with_runnel() {
    let Some(p) = Project::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/plugin.yaml"),
        env!("CARGO_BIN_EXE_faker"),
    ) else {
        return;
    };
    p.write_file(
        "pipelines/people.pipeline.yaml",
        r#"kind: Pipeline
apiVersion: runnel/v1
name: people
transforms:
  - id: fake
    type: faker.rows
    with:
      count: 250
      locale: de
      seed: 42
      fields:
        - { name: id, kind: integer, min: 1, max: 99999 }
        - { name: firstName, kind: firstName }
        - { name: lastName, kind: lastName }
        - { name: email, kind: email, from: "firstName, lastName" }
        - { name: birthDate, kind: birthDate, to: "2026-11-25" }
        - { name: amount, kind: decimal, min: 0, max: 500 }
  - id: result
    type: core.to-result
links:
  - { from: fake, to: result }
"#,
    );
    p.write_file(
        "workflows/people.workflow.yaml",
        r#"kind: Workflow
apiVersion: runnel/v1
name: people
outputs:
  - name: COUNT
    value: "{{ len(results.people.output.rows) }}"
  - name: FIRST
    value: "{{ results.people.output.rows[0].firstName }} {{ results.people.output.rows[0].lastName }}, {{ results.people.output.rows[0].email }}"
actions:
  - id: start
    type: core.start
  - id: people
    type: core.run-pipeline
    with:
      pipeline: pipelines/people.pipeline.yaml
links:
  - { from: start, to: people }
"#,
    );
    let (out, code) = p.validate();
    assert_eq!(code, 0, "{out}");
    let (out, code) = p.run("workflows/people.workflow.yaml", &[]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains(r#"output COUNT = "250""#), "{out}");
    // the same records as in testdata (seed 42): the first person
    assert!(
        out.contains(r#"output FIRST = "Christa Schreiber, christa.schreiber"#),
        "{out}"
    );

    // faker.fill pseudonymizes: the same name in two pipelines gets the same replacement.
    p.write_file(
        "pipelines/pseudo.pipeline.yaml",
        r#"kind: Pipeline
apiVersion: runnel/v1
name: pseudo
transforms:
  - id: rows
    type: core.generate-rows
    with:
      rows: "3"
      rowNumberField: n
      fields:
        - { name: name, value: Ada Lovelace }
  - id: fill
    type: faker.fill
    with:
      consistent: true
      seed: 99
      fields:
        - { name: name, kind: fullName }
        - { name: mail, kind: email }
  - id: result
    type: core.to-result
links:
  - { from: rows, to: fill }
  - { from: fill, to: result }
"#,
    );
    p.write_file(
        "workflows/pseudo.workflow.yaml",
        r#"kind: Workflow
apiVersion: runnel/v1
name: pseudo
outputs:
  - name: NAMES
    value: "{{ join(map(results.pseudo.output.rows, .name), '|') }}"
actions:
  - id: start
    type: core.start
  - id: pseudo
    type: core.run-pipeline
    with:
      pipeline: pipelines/pseudo.pipeline.yaml
links:
  - { from: start, to: pseudo }
"#,
    );
    let (out, code) = p.validate();
    assert_eq!(code, 0, "{out}");
    let (out, code) = p.run("workflows/pseudo.workflow.yaml", &[]);
    assert_eq!(code, 0, "{out}");
    let line = out
        .lines()
        .find(|l| l.contains("output NAMES = "))
        .unwrap_or_else(|| panic!("{out}"));
    let names: Vec<&str> = line
        .split("output NAMES = \"")
        .nth(1)
        .unwrap()
        .trim_end_matches('"')
        .split('|')
        .collect();
    assert_eq!(names.len(), 3, "{line}");
    assert!(names.iter().all(|n| *n == names[0] && *n != "Ada Lovelace"), "{line}");

    // faker.value in a workflow
    p.write_file(
        "workflows/value.workflow.yaml",
        r#"kind: Workflow
apiVersion: runnel/v1
name: value
outputs:
  - name: IBAN
    value: "{{ results.iban.output }}"
actions:
  - id: start
    type: core.start
  - id: iban
    type: faker.value
    with:
      kind: iban
      country: DE
      seed: "${SEED}"
links:
  - { from: start, to: iban }
"#,
    );
    let (out, code) = p.run("workflows/value.workflow.yaml", &["SEED=5"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains(r#"output IBAN = "DE"#), "{out}");

    // init: mistakes of the configuration are errors of the check, at their field.
    let path = p.dir.join("pipelines").join("people.pipeline.yaml");
    let yaml = std::fs::read_to_string(&path).unwrap();
    let yaml = yaml
        .replace("kind: decimal, min: 0, max: 500", "kind: decimal, min: 500, max: 0")
        .replace("count: 250", "count: 20000000");
    std::fs::write(&path, yaml).unwrap();
    let (out, code) = p.validate();
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("min (500) is greater than max (0)"), "{out}");
    assert!(out.contains("count must be between 0 and 10000000"), "{out}");
}
