//! Dispatches interactions onto framework commands

use crate::serenity_prelude as serenity;

/// Check if the interaction with the given name and arguments matches any framework command
fn find_matching_command<'a, 'b, U, E>(
    interaction_name: &str,
    interaction_options: &'b [serenity::ResolvedOption<'b>],
    commands: &'a [crate::Command<U, E>],
    parent_commands: &mut Vec<&'a crate::Command<U, E>>,
) -> Option<(&'a crate::Command<U, E>, &'b [serenity::ResolvedOption<'b>])> {
    commands.iter().find_map(|cmd| {
        if interaction_name != cmd.name && Some(interaction_name) != cmd.context_menu_name {
            return None;
        }

        if let Some((sub_interaction_name, sub_interaction_options)) = interaction_options
            .iter()
            .find_map(|option| match &option.value {
                serenity::ResolvedValue::SubCommand(o)
                | serenity::ResolvedValue::SubCommandGroup(o) => Some((&option.name, o)),
                _ => None,
            })
        {
            parent_commands.push(cmd);
            find_matching_command(
                sub_interaction_name,
                sub_interaction_options,
                &cmd.subcommands,
                parent_commands,
            )
        } else {
            Some((cmd, interaction_options))
        }
    })
}

/// Parses an `Interaction` into a [`crate::ApplicationContext`] using some context data.
///
/// After this, the [`crate::ApplicationContext`] should be passed into [`run_command`] or
/// [`run_autocomplete`].
fn extract_command<'a, U, E>(
    framework: crate::FrameworkContext<'a, U, E>,
    ctx: &'a serenity::Context,
    interaction: crate::CommandOrAutocompleteInteraction<'a>,
    // need to pass the following in for lifetime reasons
    has_sent_initial_response: &'a std::sync::atomic::AtomicBool,
    invocation_data: &'a tokio::sync::Mutex<Box<dyn std::any::Any + Send + Sync>>,
    options: &'a [serenity::ResolvedOption<'a>],
    parent_commands: &'a mut Vec<&'a crate::Command<U, E>>,
) -> Result<crate::ApplicationContext<'a, U, E>, crate::FrameworkError<'a, U, E>> {
    let search_result = find_matching_command(
        &interaction.data().name,
        options,
        &framework.options.commands,
        parent_commands,
    );
    let (command, leaf_interaction_options) =
        search_result.ok_or(crate::FrameworkError::UnknownInteraction {
            ctx,
            framework,
            interaction,
        })?;

    Ok(crate::ApplicationContext {
        data: framework.user_data,
        serenity_context: ctx,
        framework,
        interaction,
        args: leaf_interaction_options,
        command,
        parent_commands,
        has_sent_initial_response,
        invocation_data,
        __non_exhaustive: (),
    })
}

/// Given an interaction, finds the matching framework command and checks if the user is allowed
/// access
pub async fn extract_command_and_run_checks<'a, U, E>(
    framework: crate::FrameworkContext<'a, U, E>,
    ctx: &'a serenity::Context,
    interaction: &'a serenity::CommandInteraction,
    // Need to pass this in from outside because of lifetime issues
    has_sent_initial_response: &'a std::sync::atomic::AtomicBool,
    invocation_data: &'a tokio::sync::Mutex<Box<dyn std::any::Any + Send + Sync>>,
    // Need to pass this in from outside because of lifetime issues
    options: &'a [serenity::ResolvedOption<'a>],
    // Need to pass this in from outside because of lifetime issues
    parent_commands: &'a mut Vec<&'a crate::Command<U, E>>,
) -> Result<crate::ApplicationContext<'a, U, E>, crate::FrameworkError<'a, U, E>> {
    let ctx = extract_command(
        framework,
        ctx,
        crate::CommandOrAutocompleteInteraction::Command(interaction),
        has_sent_initial_response,
        invocation_data,
        options,
        parent_commands,
    )?;
    super::common::check_permissions_and_cooldown(ctx.into()).await?;
    Ok(ctx)
}

/// Given the extracted application command data from [`extract_command`], runs the command,
/// including all the before and after code like checks.
async fn run_command<U, E>(
    ctx: crate::ApplicationContext<'_, U, E>,
) -> Result<(), crate::FrameworkError<'_, U, E>> {
    super::common::check_permissions_and_cooldown(ctx.into()).await?;

    (ctx.framework.options.pre_command)(crate::Context::Application(ctx)).await;

    // Check which interaction type we received and grab the command action and, if context menu,
    // the resolved click target, and execute the action
    let command_structure_mismatch_error = crate::FrameworkError::CommandStructureMismatch {
        ctx,
        description: "received interaction type but command contained no \
                matching action or interaction contained no matching context menu object",
    };
    let action_result = match ctx.interaction.data().kind {
        serenity::CommandType::ChatInput => {
            let action = ctx
                .command
                .slash_action
                .ok_or(command_structure_mismatch_error)?;
            action(ctx).await
        }
        serenity::CommandType::User => {
            match (ctx.command.context_menu_action, interaction.data.target()) {
                (
                    Some(crate::ContextMenuCommandAction::User(action)),
                    Some(serenity::ResolvedTarget::User(user, _)),
                ) => action(ctx, user.clone()).await,
                _ => return Err(command_structure_mismatch_error),
            }
        }
        serenity::CommandType::Message => {
            match (ctx.command.context_menu_action, interaction.data.target()) {
                (
                    Some(crate::ContextMenuCommandAction::Message(action)),
                    Some(serenity::ResolvedTarget::Message(message)),
                ) => action(ctx, message.clone()).await,
                _ => return Err(command_structure_mismatch_error),
            }
        }
        other => {
            log::warn!("unknown interaction command type: {:?}", other);
            return Ok(());
        }
    };
    action_result?;

    (ctx.framework.options.post_command)(crate::Context::Application(ctx)).await;

    Ok(())
}

/// Dispatches this interaction onto framework commands, i.e. runs the associated command
pub async fn dispatch_interaction<'a, U, E>(
    framework: crate::FrameworkContext<'a, U, E>,
    ctx: &'a serenity::Context,
    interaction: &'a serenity::CommandInteraction,
    // Need to pass this in from outside because of lifetime issues
    has_sent_initial_response: &'a std::sync::atomic::AtomicBool,
    // Need to pass this in from outside because of lifetime issues
    invocation_data: &'a tokio::sync::Mutex<Box<dyn std::any::Any + Send + Sync>>,
    // Need to pass this in from outside because of lifetime issues
    options: &'a [serenity::ResolvedOption<'a>],
    // Need to pass this in from outside because of lifetime issues
    parent_commands: &'a mut Vec<&'a crate::Command<U, E>>,
) -> Result<(), crate::FrameworkError<'a, U, E>> {
    let ctx = extract_command(
        framework,
        ctx,
        crate::CommandOrAutocompleteInteraction::Autocomplete(interaction),
        has_sent_initial_response,
        invocation_data,
        options,
        parent_commands,
    )?;

    crate::catch_unwind_maybe(run_command(ctx))
        .await
        .map_err(|payload| crate::FrameworkError::CommandPanic {
            payload,
            ctx: ctx.into(),
        })??;

    Ok(())
}

/// Given the extracted application command data from [`extract_command`], runs the autocomplete
/// callbacks, including all the before and after code like checks.
async fn run_autocomplete<U, E>(
    ctx: crate::ApplicationContext<'_, U, E>,
) -> Result<(), crate::FrameworkError<'_, U, E>> {
    super::common::check_permissions_and_cooldown(ctx.into()).await?;

    // Find which parameter is focused by the user
    let (focused_option_name, partial_input) = match ctx.args.iter().find_map(|o| match &o.value {
        serenity::ResolvedValue::Autocomplete { value, .. } => Some((&o.name, value)),
        _ => None,
    }) {
        Some(x) => x,
        None => {
            log::warn!("no option is focused in autocomplete interaction");
            return Ok(());
        }
    };

    // Find the matching parameter from our Command object
    let parameters = &ctx.command.parameters;
    let focused_parameter = parameters
        .iter()
        .find(|p| p.name == *focused_option_name)
        .ok_or(crate::FrameworkError::CommandStructureMismatch {
            ctx,
            description: "focused autocomplete parameter name not recognized",
        })?;

    // Only continue if this parameter supports autocomplete and Discord has given us a partial value
    let autocomplete_callback = match focused_parameter.autocomplete_callback {
        Some(x) => x,
        _ => return Ok(()),
    };

    #[allow(unused_imports)]
    use ::serenity::json::prelude::*; // as_str() access via trait for simd-json

    // Generate an autocomplete response
    let autocomplete_response = match autocomplete_callback(ctx, partial_input).await {
        Ok(x) => x,
        Err(e) => {
            log::warn!("couldn't generate autocomplete response: {}", e);
            return Ok(());
        }
    };

    let interaction = match ctx.interaction {
        crate::ApplicationCommandOrAutocompleteInteraction::Autocomplete(x) => x,
        _ => {
            log::warn!("a non-autocomplete interaction was given to run_autocomplete()");
            return Ok(());
        }
    };

    // Send the generates autocomplete response
    if let Err(e) = interaction
        .create_response(
            &ctx.discord.http,
            serenity::CreateInteractionResponse::Autocomplete(autocomplete_response),
        )
        .await
    {
        log::warn!("couldn't send autocomplete response: {}", e);
    }

    Ok(())
}

/// Dispatches this interaction onto framework commands, i.e. runs the associated autocomplete
/// callback
pub async fn dispatch_autocomplete<'a, U, E>(
    framework: crate::FrameworkContext<'a, U, E>,
    ctx: &'a serenity::Context,
    interaction: &'a serenity::AutocompleteInteraction,
    // Need to pass this in from outside because of lifetime issues
    has_sent_initial_response: &'a std::sync::atomic::AtomicBool,
    // Need to pass this in from outside because of lifetime issues
    invocation_data: &'a tokio::sync::Mutex<Box<dyn std::any::Any + Send + Sync>>,
    // Need to pass this in from outside because of lifetime issues
    parent_commands: &'a mut Vec<&'a crate::Command<U, E>>,
) -> Result<(), crate::FrameworkError<'a, U, E>> {
    let ctx = extract_command(
        framework,
        ctx,
        crate::ApplicationCommandOrAutocompleteInteraction::Autocomplete(interaction),
        has_sent_initial_response,
        invocation_data,
        parent_commands,
    )?;

    crate::catch_unwind_maybe(run_autocomplete(ctx))
        .await
        .map_err(|payload| crate::FrameworkError::CommandPanic {
            payload,
            ctx: ctx.into(),
        })??;

    Ok(())
}
