//! Hosts just the `AutocompleteChoice` type. This type will probably move somewhere else

use serenity::all as serenity;

/// A single autocomplete choice, displayed in Discord UI
///
/// This type can be returned by functions set via the `#[autocomplete = ]` attribute on slash
/// command parameters.
///
/// For more information, see the autocomplete.rs file in the `framework_usage` example
pub struct AutocompleteChoice<T> {
    /// Name of the choice, displayed in the Discord UI
    pub name: String,
    /// Value of the choice, sent to the bot
    pub value: T,
}

impl<T> AutocompleteChoice<T> {
    /// Converts this type to the serenity equivalent in order to pass it to serenity's API endpoint
    /// functions.
    pub fn to_serenity(self) -> serenity::AutocompleteChoice
    where
        T: Into<serenity::json::Value>,
    {
        serenity::AutocompleteChoice::new(self.name, self.value)
    }
}

impl<T: ToString> From<T> for AutocompleteChoice<T> {
    fn from(value: T) -> Self {
        Self {
            name: value.to_string(),
            value,
        }
    }
}
