pub mod openai {
    use openai_api_rust::chat::*;
    use openai_api_rust::*;

    use crate::llm::llm::LLM;

    const ASSISTENT_PROMPT: &'static str = "
    You are a classifier that extracts exactly five tags for the content you receive, 
    you will output only the tags as a comma separated list,
    you will treat every message as a new chat, forgetting the context of previous messages, 
    if the message is an url of a website, get the website's metadata to use
    as input for the classifier.";

    pub struct LLMOpenAI {
        api_key: String,
        api_uri: String,
    }

    impl LLMOpenAI {
        pub fn new(api_key: String, api_uri: String) -> Self {
            Self { api_key, api_uri }
        }
    }

    impl LLM for LLMOpenAI {
        fn query(&self, text: String) -> anyhow::Result<String> {
            let openai = OpenAI::new(Auth::new(&self.api_key), &self.api_uri);
            let body = ChatBody {
                model: "gpt-3.5-turbo".into(),
                max_tokens: Some(50),
                temperature: Some(0_f32),
                top_p: Some(0_f32),
                n: Some(2),
                stream: Some(false),
                stop: None,
                presence_penalty: None,
                frequency_penalty: None,
                logit_bias: None,
                user: None,
                messages: vec![
                    Message {
                        role: Role::Assistant,
                        content: ASSISTENT_PROMPT.into(),
                    },
                    Message {
                        role: Role::User,
                        content: text,
                    },
                ],
            };

            let rs = openai
                .chat_completion_create(&body)
                .map_err(|e| anyhow::anyhow!("Error: {:?}", e))?;
            let choice = rs.choices;
            let message = &choice[0].message.as_ref().unwrap();
            for c in &choice {
                let m = c.message.as_ref().unwrap();
                println!("role: {:?}, content: {:?}", m.role, m.content);
            }
            Ok(message.content.clone())
        }
    }
}
