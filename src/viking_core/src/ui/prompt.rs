use std::sync::Once;

use crate::ui;

/// Show an interactive selection prompt if `options` contains more than 1 choice
pub fn interactive_select<'a, T, FnFormat: Fn(&T) -> String>(
    prompt: &str,
    options: &'a [T],
    fn_format: FnFormat,
) -> cu::Result<Option<&'a T>> {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/ui.rs#L71
    match options {
        [] => return Ok(None),
        [_only_match] => return Ok(Some(&options[0])),
        _ => {}
    }
    ui::clear_terminal();

    let formatted_options = options.iter().map(fn_format).collect();

    init_inquire();
    let i = inquire::Select::new(prompt, formatted_options)
        .with_starting_cursor(0)
        .with_page_size(crossterm::terminal::size()?.1 as usize / 2)
        .raw_prompt()?
        .index;
    Ok(Some(&options[i]))
}

/// Show a confirmation prompt. `Ok(())` means the user chose to proceed
///
/// ```text
/// {prompt} (Y/n)
/// [{help}]
/// ```
pub fn confirm(prompt: &str, default_value: bool, help: Option<&str>) -> cu::Result<()> {
    init_inquire();
    let mut confirm = inquire::Confirm::new(prompt).with_default(default_value);
    if let Some(help) = help {
        confirm = confirm.with_help_message(help);
    }
    if !confirm.prompt()? {
        cu::bail!("Operation was cancelled by the user");
    }
    Ok(())
}

fn init_inquire() {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/ui.rs#L56
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let mut config = inquire::ui::RenderConfig::default();
        config.prompt.att |= inquire::ui::Attributes::BOLD;
        inquire::set_global_render_config(config);
    });
}
