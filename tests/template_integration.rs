use cargo_image_runner::config::Config;
use cargo_image_runner::image::TemplateProcessor;
use std::collections::HashMap;

#[test]
fn test_template_with_limine_config() {
    let mut vars = HashMap::new();
    vars.insert("TIMEOUT".to_string(), "5".to_string());
    vars.insert(
        "EXECUTABLE_NAME".to_string(),
        "my-kernel".to_string(),
    );

    let template = r#"timeout: {{TIMEOUT}}

/My Kernel
    protocol: limine
    kernel_path: boot():/boot/{{EXECUTABLE_NAME}}
"#;

    let result = TemplateProcessor::process(template, &vars).unwrap();
    assert_eq!(
        result,
        "timeout: 5\n\n/My Kernel\n    protocol: limine\n    kernel_path: boot():/boot/my-kernel\n"
    );
}

#[test]
fn test_template_with_all_builtin_vars() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("test-kernel");
    std::fs::write(&exe, b"fake").unwrap();

    let ctx = cargo_image_runner::core::Context::new(
        Config::default(),
        dir.path().to_path_buf(),
        exe.clone(),
    )
    .unwrap();

    let template = "exe={{EXECUTABLE}} name={{EXECUTABLE_NAME}} root={{WORKSPACE_ROOT}} out={{OUTPUT_DIR}} test={{IS_TEST}}";
    let result = TemplateProcessor::process(template, &ctx.template_vars).unwrap();

    assert!(result.contains(&format!("exe={}", exe.display())));
    assert!(result.contains("name=test-kernel"));
    assert!(result.contains(&format!("root={}", dir.path().display())));
    assert!(result.contains(&format!(
        "out={}",
        dir.path().join("target/image-runner/output").display()
    )));
    assert!(result.contains("test=0"));
}

#[test]
fn test_template_dollar_syntax() {
    let mut vars = HashMap::new();
    vars.insert("VERSION".to_string(), "1.0".to_string());

    let result = TemplateProcessor::process("v$VERSION", &vars).unwrap();
    assert_eq!(result, "v1.0");
}

#[test]
fn test_template_user_variables_from_config() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("kernel");
    std::fs::write(&exe, b"fake").unwrap();

    let mut config = Config::default();
    config
        .variables
        .insert("CUSTOM_VAR".to_string(), "custom_value".to_string());

    let ctx = cargo_image_runner::core::Context::new(
        config,
        dir.path().to_path_buf(),
        exe,
    )
    .unwrap();

    let result =
        TemplateProcessor::process("{{CUSTOM_VAR}}", &ctx.template_vars).unwrap();
    assert_eq!(result, "custom_value");
}
