use super::*;

#[test]
fn the_current_defaults_see_and_the_text_models_do_not() {
    for m in [
        "claude-sonnet-5-5",
        "anthropic/claude-opus-5",
        "gpt-6",
        "openai/gpt-4o",
        "o3-mini",
        "gemini-3-pro",
        "qwen-vl-max",
    ] {
        assert!(sees(m), "{m}");
    }
    for m in [
        "qwen-max",
        "qwen-plus",
        "deepseek-chat",
        "deepseek/deepseek-r1",
        "turbo-o3x",
        "nvidia/llama-3.1-nemotron",
    ] {
        assert!(!sees(m), "{m}");
    }
}

#[test]
fn a_text_model_is_told_in_the_images_place() {
    let img =
        super::super::Image::from_bytes("a.png", &super::super::image::fake_png(1, 1)).unwrap();
    let one = [img];
    let (shown, notes) = split("deepseek-chat", &one);
    assert!(shown.is_empty());
    assert_eq!(
        notes,
        ["[image omitted: a.png, this model reads text only]"]
    );
    assert_eq!(split("claude-sonnet-5-5", &one).0.len(), 1);
}
