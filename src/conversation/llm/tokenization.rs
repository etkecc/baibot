use tiktoken_rs::CoreBPE;
use tiktoken_rs::bpe_for_tokenizer;
use tiktoken_rs::tokenizer;

use super::{Author, Message, MessageContent};

/// How to count the tokens in a conversation when trimming it to fit the context window.
pub enum TokenEstimate<'a> {
    /// Count via the [tiktoken-rs](https://github.com/zurawiki/tiktoken-rs) library.
    /// Accurate for OpenAI models; every other model falls back to the gpt-4
    /// tokenizer, which misreads it (badly so for non-English text). Use this only
    /// for the OpenAI provider.
    Tiktoken(&'a str),

    /// Provider-neutral approximation that needs no per-model tokenizer. Expect it
    /// to land within roughly 10-20% of the real count for typical text, leaning
    /// slightly high: over-counting trims a little extra history, while
    /// under-counting would overflow the model's real context window. Use this for
    /// every non-OpenAI provider.
    Approximate,
}

fn get_bpe_for_model(model: &str) -> &'static CoreBPE {
    let tokenizer = tokenizer::get_tokenizer(model)
        .or_else(|| tokenizer::get_tokenizer("gpt-4"))
        .unwrap();

    bpe_for_tokenizer(tokenizer).unwrap()
}

pub fn shorten_messages_list_to_context_size(
    estimate: TokenEstimate<'_>,
    prompt_message: &Option<Message>,
    mut messages: Vec<Message>,
    max_response_tokens: Option<u32>,
    max_context_tokens: u32,
) -> Vec<Message> {
    // Loading the tiktoken data is expensive, so we resolve the counter once up
    // front and reuse it for every message.
    let tiktoken = match estimate {
        TokenEstimate::Tiktoken(model) => Some((get_bpe_for_model(model), model)),
        TokenEstimate::Approximate => None,
    };
    let count = |message: &Message| match tiktoken {
        Some((bpe, model)) => tiktoken_token_size_for_message(bpe, model, message),
        None => approximate_token_size_for_message(message),
    };

    // We want to retain the prompt in all cases, so we always count it first.
    // We also always reserve enough tokens for the maximum response we expect.
    let mut current_context_length: u32 = if let Some(prompt_message) = prompt_message {
        count(prompt_message) + max_response_tokens.unwrap_or(0)
    } else {
        0
    };

    messages.reverse();

    let mut messages_to_keep: Vec<Message> = Vec::new();

    for message in messages {
        let tokens_for_message = count(&message);

        if current_context_length + tokens_for_message > max_context_tokens {
            break;
        }

        current_context_length += tokens_for_message;

        messages_to_keep.push(message);
    }

    // Cut on a turn boundary: the loop may stop right after an assistant reply
    // whose triggering user message did not fit, which would leave the kept window
    // starting on an orphaned reply. `messages_to_keep` is newest-first here, so
    // the oldest kept messages are at the end; drop any trailing assistant messages
    // until the window begins at the start of a turn (a user message).
    while matches!(
        messages_to_keep.last().map(|message| &message.author),
        Some(Author::Assistant)
    ) {
        messages_to_keep.pop();
    }

    messages_to_keep.reverse();

    messages_to_keep
}

/// Token size of a message via tiktoken, for a preloaded CoreBPE object.
/// Accurate only for OpenAI models (see [`TokenEstimate::Tiktoken`]).
fn tiktoken_token_size_for_message(bpe: &CoreBPE, model: &str, message: &Message) -> u32 {
    let (tokens_per_message, tokens_per_name) = if model.starts_with("gpt-3.5") {
        (
            4,  // every message follows <im_start>{role/name}\n{content}<im_end>\n
            -1, // if there's a name, the role is omitted
        )
    } else {
        (3, 1)
    };

    let role_length = match message.author {
        Author::Assistant => bpe.encode_with_special_tokens("assistant").len() as i32,
        Author::User => bpe.encode_with_special_tokens("user").len() as i32,
        Author::Prompt => bpe.encode_with_special_tokens("system").len() as i32,
    };

    let text_length = match &message.content {
        MessageContent::Text(text) => bpe.encode_with_special_tokens(text).len() as i32,
        MessageContent::Image(..) => 0,
        MessageContent::File(..) => 0,
    };

    (text_length + role_length + tokens_per_message + tokens_per_name) as u32
}

/// ASCII text averages about four characters per token.
const ASCII_TOKENS_PER_CHAR: f32 = 0.25;

/// Non-ASCII scripts (Cyrillic, CJK, and others) pack more information per
/// character: real tokenizers land around two characters per token for them, so
/// each counts as half a token. CJK runs a touch denser than that, so its estimate
/// can read slightly low, still within the tolerance this approximation targets.
const WIDE_TOKENS_PER_CHAR: f32 = 0.5;

/// Structural per-message overhead (role marker plus message framing), mirroring
/// the small constant the tiktoken path adds.
const APPROX_TOKENS_PER_MESSAGE: u32 = 4;

/// Provider-neutral, tokenizer-free token size of a message
/// (see [`TokenEstimate::Approximate`]).
fn approximate_token_size_for_message(message: &Message) -> u32 {
    let text_tokens = match &message.content {
        MessageContent::Text(text) => approximate_token_size_for_text(text),
        // Images and files are not counted as text, matching the tiktoken path.
        MessageContent::Image(..) | MessageContent::File(..) => 0,
    };

    text_tokens + APPROX_TOKENS_PER_MESSAGE
}

/// Rough token estimate for a piece of text, with no tokenizer.
///
/// ASCII characters count as a quarter-token each (~4 chars/token); characters
/// outside ASCII count as half a token each (~2 chars/token), matching how real
/// tokenizers treat Cyrillic and CJK. Weighting non-ASCII up keeps the estimate
/// from badly under-counting non-English text, the case the tiktoken fallback gets
/// most wrong.
fn approximate_token_size_for_text(text: &str) -> u32 {
    let mut estimate = 0.0_f32;

    for character in text.chars() {
        estimate += if character.is_ascii() {
            ASCII_TOKENS_PER_CHAR
        } else {
            WIDE_TOKENS_PER_CHAR
        };
    }

    estimate.ceil() as u32
}

pub mod test {
    #[test]
    fn message_size_counting_works() {
        let model = "gpt-4";

        let bpe = super::get_bpe_for_model(model);

        let message = super::Message {
            author: super::Author::User,
            sender_id: None,
            content: super::MessageContent::Text("Hello there!".to_string()),
            timestamp: chrono::Utc::now(),
        };

        let tokens = super::tiktoken_token_size_for_message(bpe, model, &message);

        assert_eq!(8, tokens);
    }

    #[test]
    fn shortening_works_with_english() {
        let model = "gpt-4";

        let bpe = super::get_bpe_for_model(model);

        let max_response_tokens_value: u32 = 5;
        let max_response_tokens: Option<u32> = Some(max_response_tokens_value);

        let prompt = super::Message {
            author: super::Author::Prompt,
            sender_id: None,
            content: super::MessageContent::Text("You are a bot!".to_string()),
            timestamp: chrono::Utc::now(),
        };
        let prompt_length = 10;

        assert_eq!(
            prompt_length,
            super::tiktoken_token_size_for_message(bpe, model, &prompt)
        );

        let mut conversation_messages = Vec::new();

        let first = super::Message {
            author: super::Author::User,
            sender_id: None,
            content: super::MessageContent::Text("Hello there!".to_string()),
            timestamp: chrono::Utc::now(),
        };
        let first_length = 8;

        assert_eq!(
            first_length,
            super::tiktoken_token_size_for_message(bpe, model, &first)
        );

        conversation_messages.push(first);

        let second = super::Message {
            author: super::Author::Assistant,
            sender_id: None,
            content: super::MessageContent::Text("Hello!".to_string()),
            timestamp: chrono::Utc::now(),
        };
        let second_length = 7;

        assert_eq!(
            second_length,
            super::tiktoken_token_size_for_message(bpe, model, &second)
        );

        conversation_messages.push(second);

        let third = super::Message {
            author: super::Author::User,
            sender_id: None,
            content: super::MessageContent::Text(
                "This is the 3rd message in this conversation. It shall be preserved.".to_owned(),
            ),
            timestamp: chrono::Utc::now(),
        };
        let third_length = 21;

        assert_eq!(
            third_length,
            super::tiktoken_token_size_for_message(bpe, model, &third)
        );

        conversation_messages.push(third.clone());

        let forth = super::Message {
            author: super::Author::Assistant,
            sender_id: None,
            content: super::MessageContent::Text(
                "This is yet another message that shall be preserved.".to_owned(),
            ),
            timestamp: chrono::Utc::now(),
        };
        let forth_length = 15;

        assert_eq!(
            forth_length,
            super::tiktoken_token_size_for_message(bpe, model, &forth)
        );

        conversation_messages.push(forth.clone());

        assert_eq!(4, conversation_messages.len());

        let new_conversation_messages = super::shorten_messages_list_to_context_size(
            super::TokenEstimate::Tiktoken(model),
            &Some(prompt),
            conversation_messages,
            max_response_tokens,
            prompt_length + max_response_tokens_value + forth_length + third_length,
        );

        assert_eq!(2, new_conversation_messages.len());

        assert_eq!(
            new_conversation_messages.first().unwrap().content,
            third.content
        );

        assert_eq!(
            new_conversation_messages.last().unwrap().content,
            forth.content
        );
    }

    #[test]
    fn shortening_works_with_japanese() {
        let model = "gpt-4";

        let bpe = super::get_bpe_for_model(model);

        let max_response_tokens_value: u32 = 5;
        let max_response_tokens: Option<u32> = Some(max_response_tokens_value);

        let prompt = super::Message {
            author: super::Author::User,
            sender_id: None,
            content: super::MessageContent::Text("あなたはボットです。".to_string()),
            timestamp: chrono::Utc::now(),
        };
        let prompt_length = 14;

        assert_eq!(
            prompt_length,
            super::tiktoken_token_size_for_message(bpe, model, &prompt)
        );

        let mut conversation_messages = Vec::new();

        let first = super::Message {
            author: super::Author::User,
            sender_id: None,
            content: super::MessageContent::Text("こんにちは!".to_string()),
            timestamp: chrono::Utc::now(),
        };
        let first_length = 7;

        assert_eq!(
            first_length,
            super::tiktoken_token_size_for_message(bpe, model, &first)
        );

        conversation_messages.push(first);

        let second = super::Message {
            author: super::Author::Assistant,
            sender_id: None,
            content: super::MessageContent::Text("こんにちは。今日は元気ですか。".to_string()),
            timestamp: chrono::Utc::now(),
        };
        let second_length = 15;

        assert_eq!(
            second_length,
            super::tiktoken_token_size_for_message(bpe, model, &second)
        );

        conversation_messages.push(second);

        let third = super::Message {
            author: super::Author::User,
            sender_id: None,
            content: super::MessageContent::Text(
                "これは第3のメッセージなので、保存されます。".to_string(),
            ),
            timestamp: chrono::Utc::now(),
        };
        let third_length = 22;

        assert_eq!(
            third_length,
            super::tiktoken_token_size_for_message(bpe, model, &third)
        );

        conversation_messages.push(third.clone());

        let forth = super::Message {
            author: super::Author::Assistant,
            sender_id: None,
            content: super::MessageContent::Text(
                "これはもう一つの保存されますメッセージです。".to_string(),
            ),
            timestamp: chrono::Utc::now(),
        };
        let forth_length = 21;

        assert_eq!(
            forth_length,
            super::tiktoken_token_size_for_message(bpe, model, &forth)
        );

        conversation_messages.push(forth.clone());

        assert_eq!(4, conversation_messages.len());

        let new_conversation_messages = super::shorten_messages_list_to_context_size(
            super::TokenEstimate::Tiktoken(model),
            &Some(prompt),
            conversation_messages,
            max_response_tokens,
            prompt_length + max_response_tokens_value + forth_length + third_length,
        );

        assert_eq!(2, new_conversation_messages.len());

        assert_eq!(
            new_conversation_messages.first().unwrap().content,
            third.content
        );

        assert_eq!(
            new_conversation_messages.last().unwrap().content,
            forth.content
        );
    }

    #[test]
    fn approximate_counting_weights_ascii_and_wide_scripts() {
        // 12 ASCII characters at ~4 chars/token = 3 text tokens.
        assert_eq!(3, super::approximate_token_size_for_text("Hello there!"));

        // 5 CJK characters at ~0.5 token/char = 3 text tokens. The ASCII rate would
        // have under-counted these to 2, the failure mode this path avoids.
        assert_eq!(3, super::approximate_token_size_for_text("こんにちは"));

        let message = super::Message {
            author: super::Author::User,
            sender_id: None,
            content: super::MessageContent::Text("Hello there!".to_string()),
            timestamp: chrono::Utc::now(),
        };

        // 3 text tokens plus the per-message overhead (4).
        assert_eq!(7, super::approximate_token_size_for_message(&message));
    }

    #[test]
    fn approximate_shortening_trims_to_budget() {
        let prompt = super::Message {
            author: super::Author::Prompt,
            sender_id: None,
            content: super::MessageContent::Text("You are a bot!".to_string()),
            timestamp: chrono::Utc::now(),
        };

        let older = super::Message {
            author: super::Author::User,
            sender_id: None,
            content: super::MessageContent::Text("This is the older message.".to_string()),
            timestamp: chrono::Utc::now(),
        };
        let newer = super::Message {
            // A user message, so it is a valid window start: keeping a lone
            // assistant reply would be an orphan and get trimmed (see
            // `shortening_cuts_on_a_turn_boundary`).
            author: super::Author::User,
            sender_id: None,
            content: super::MessageContent::Text("This is the newer message.".to_string()),
            timestamp: chrono::Utc::now(),
        };

        // Budget room for the prompt and only the newest message.
        let max_context_tokens = super::approximate_token_size_for_message(&prompt)
            + super::approximate_token_size_for_message(&newer);

        let new_conversation_messages = super::shorten_messages_list_to_context_size(
            super::TokenEstimate::Approximate,
            &Some(prompt),
            vec![older, newer.clone()],
            None,
            max_context_tokens,
        );

        assert_eq!(1, new_conversation_messages.len());
        assert_eq!(
            new_conversation_messages.first().unwrap().content,
            newer.content
        );
    }

    #[test]
    fn shortening_cuts_on_a_turn_boundary() {
        // A four-message conversation of two full turns. All four messages are the
        // same length, so they cost the same number of tokens.
        let prompt = super::Message {
            author: super::Author::Prompt,
            sender_id: None,
            content: super::MessageContent::Text("system".to_string()),
            timestamp: chrono::Utc::now(),
        };

        let user_one = super::Message {
            author: super::Author::User,
            sender_id: None,
            content: super::MessageContent::Text("user msg 1".to_string()),
            timestamp: chrono::Utc::now(),
        };
        let asst_one = super::Message {
            author: super::Author::Assistant,
            sender_id: None,
            content: super::MessageContent::Text("asst msg 1".to_string()),
            timestamp: chrono::Utc::now(),
        };
        let user_two = super::Message {
            author: super::Author::User,
            sender_id: None,
            content: super::MessageContent::Text("user msg 2".to_string()),
            timestamp: chrono::Utc::now(),
        };
        let asst_two = super::Message {
            author: super::Author::Assistant,
            sender_id: None,
            content: super::MessageContent::Text("asst msg 2".to_string()),
            timestamp: chrono::Utc::now(),
        };

        let per_message = super::approximate_token_size_for_message(&user_one);
        // Budget fits the prompt plus three messages. By raw token budget the loop
        // would keep asst_two, user_two, and asst_one, but asst_one's own user
        // message (user_one) does not fit, so it must be dropped too rather than
        // left as an orphaned reply.
        let max_context_tokens =
            super::approximate_token_size_for_message(&prompt) + (per_message * 3);

        let kept = super::shorten_messages_list_to_context_size(
            super::TokenEstimate::Approximate,
            &Some(prompt),
            vec![user_one, asst_one, user_two.clone(), asst_two.clone()],
            None,
            max_context_tokens,
        );

        // Only the last whole turn survives; the orphaned asst_one is dropped.
        assert_eq!(2, kept.len());
        assert_eq!(kept.first().unwrap().content, user_two.content);
        assert_eq!(kept.last().unwrap().content, asst_two.content);
    }
}
