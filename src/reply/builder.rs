//! The builder to create a new reply

use crate::serenity_prelude as serenity;

/// Message builder that abstracts over prefix and application command responses
#[derive(Default, Clone)]
pub struct CreateReply {
    /// Message content.
    pub content: Option<String>,
    /// Embeds, if present.
    pub embeds: Vec<serenity::CreateEmbed>,
    /// Message attachments.
    pub attachments: Vec<serenity::CreateAttachment>,
    /// Whether the message is ephemeral (only has an effect in application commands)
    ///
    /// If None, it's initialized to [`crate::Command::ephemeral`]
    pub ephemeral: Option<bool>,
    /// Message components, that is, buttons and select menus.
    pub components: Option<Vec<serenity::CreateActionRow>>,
    /// The allowed mentions for the message.
    pub allowed_mentions: Option<serenity::CreateAllowedMentions>,
    /// Whether this message is an inline reply.
    pub reply: bool,
}

impl CreateReply {
    /// Creates a new blank [`CreateReply`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the content of the message.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Adds an embed to the message.
    ///
    /// Existing embeds are kept.
    pub fn embed(mut self, b: serenity::CreateEmbed) -> Self {
        self.embeds.push(b);
        self
    }

    /// Set components (buttons and select menus) for this message.
    ///
    /// Any previously set components will be overwritten.
    pub fn components(mut self, b: Vec<serenity::CreateActionRow>) -> Self {
        self.components = Some(b);
        self
    }

    /// Add an attachment.
    ///
    /// This will not have an effect in a slash command's initial response!
    pub fn attachment(mut self, attachment: serenity::CreateAttachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Toggles whether the message is an ephemeral response (only invoking user can see it).
    ///
    /// This only has an effect in slash commands!
    pub fn ephemeral(mut self, ephemeral: bool) -> Self {
        self.ephemeral = Some(ephemeral);
        self
    }

    /// Set the allowed mentions for the message.
    ///
    /// See [`serenity::CreateAllowedMentions`] for more information.
    pub fn allowed_mentions(mut self, b: serenity::CreateAllowedMentions) -> Self {
        self.allowed_mentions = Some(b);
        self
    }

    /// Makes this message an inline reply to another message like [`serenity::Message::reply`]
    /// (prefix-only, because slash commands are always inline replies anyways).
    ///
    /// To disable the ping, set [`Self::allowed_mentions`] with
    /// [`serenity::CreateAllowedMentions::replied_user`] set to false.
    pub fn reply(mut self, reply: bool) -> Self {
        self.reply = reply;
        self
    }

    /// Utility function that sets up a CreateReply builder with data that it always has (as
    /// configured in the framework or the command)
    ///
    /// Invoked in every place where a CreateReply is accepted and sent to Discord
    pub(crate) fn complete_from_ctx<U, E>(mut self, ctx: crate::Context<'_, U, E>) -> Self {
        self.ephemeral.get_or_insert(ctx.command().ephemeral);
        if let Some(allowed_mentions) = ctx.framework().options().allowed_mentions.clone() {
            self.allowed_mentions.get_or_insert(allowed_mentions);
        }
        if let Some(callback) = ctx.framework().options().reply_callback {
            self = callback(ctx, self);
        }
        self
    }
}

/// Methods to create a message builder from any type from this [`CreateReply`]. Used by poise
/// internally to actually send a response to Discord
impl CreateReply {
    /// Serialize this response builder to a [`serenity::CreateInteractionResponseMessage`]
    pub fn to_slash_initial_response(self) -> serenity::CreateInteractionResponseMessage {
        let mut f = serenity::CreateInteractionResponseMessage::default();
        let crate::CreateReply {
            content,
            embeds,
            attachments,
            components,
            ephemeral,
            allowed_mentions,
            reply: _, // can't reply to a message in interactions
        } = self;

        if let Some(content) = content {
            f = f.content(content);
        }
        f = f.embeds(embeds);
        if let Some(allowed_mentions) = allowed_mentions {
            f = f.allowed_mentions(allowed_mentions);
        }
        if let Some(components) = components {
            f = f.components(components);
        }
        f = f.ephemeral(ephemeral.unwrap_or(false));
        f = f.add_files(attachments);

        f
    }

    /// Serialize this response builder to a [`serenity::CreateInteractionResponseFollowup`]
    pub fn to_slash_followup_response(self) -> serenity::CreateInteractionResponseFollowup {
        let mut f = serenity::CreateInteractionResponseFollowup::default();
        let crate::CreateReply {
            content,
            embeds,
            attachments,
            components,
            ephemeral,
            allowed_mentions,
            reply: _,
        } = self;

        if let Some(content) = content {
            f = f.content(content);
        }
        f = f.embeds(embeds);
        if let Some(components) = components {
            f = f.components(components);
        }
        if let Some(allowed_mentions) = allowed_mentions {
            f = f.allowed_mentions(allowed_mentions);
        }
        f = f.ephemeral(ephemeral.unwrap_or(false));
        f = f.add_files(attachments);

        f
    }

    /// Serialize this response builder to a [`serenity::EditInteractionResponse`]
    pub fn to_slash_initial_response_edit(self) -> serenity::EditInteractionResponse {
        let mut f = serenity::EditInteractionResponse::default();
        let crate::CreateReply {
            content,
            embeds,
            attachments: _, // no support for attachment edits in serenity yet
            components,
            ephemeral: _, // can't edit ephemerality in retrospect
            allowed_mentions,
            reply: _,
        } = self;

        if let Some(content) = content {
            f = f.content(content);
        }
        f = f.embeds(embeds);
        if let Some(components) = components {
            f = f.components(components);
        }
        if let Some(allowed_mentions) = allowed_mentions {
            f = f.allowed_mentions(allowed_mentions);
        }

        f
    }

    /// Serialize this response builder to a [`serenity::EditMessage`]
    pub fn to_prefix_edit(self) -> serenity::EditMessage {
        let mut f = serenity::EditMessage::default();
        let crate::CreateReply {
            content,
            embeds,
            attachments,
            components,
            ephemeral: _, // not supported in prefix
            allowed_mentions,
            reply: _, // can't edit reference message afterwards
        } = self;

        if let Some(content) = content {
            f = f.content(content);
        }
        f = f.add_embeds(embeds);
        for attachment in attachments {
            f = f.attachment(attachment);
        }

        if let Some(allowed_mentions) = allowed_mentions {
            f = f.allowed_mentions(allowed_mentions);
        }

        if let Some(components) = components {
            f = f.components(components);
        }

        f
    }

    /// Serialize this response builder to a [`serenity::CreateMessage`]
    pub fn to_prefix(self, invocation_message: &serenity::Message) -> serenity::CreateMessage {
        let mut m = serenity::CreateMessage::default();
        let crate::CreateReply {
            content,
            embeds,
            attachments,
            components,
            ephemeral: _, // not supported in prefix
            allowed_mentions,
            reply,
        } = self;

        if let Some(content) = content {
            m = m.content(content);
        }
        m = m.embeds(embeds);
        if let Some(allowed_mentions) = allowed_mentions {
            m = m.allowed_mentions(allowed_mentions);
        }
        if let Some(components) = components {
            m = m.components(components);
        }
        if reply {
            m = m.reference_message(invocation_message);
        }

        for attachment in attachments {
            m = m.add_file(attachment);
        }

        m
    }
}
