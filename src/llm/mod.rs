pub mod llm {
    use config::Config;
    use openai_api_rust::chat::*;
    use openai_api_rust::*;

    const ASSISTENT_PROMPT: &'static str = "
    You are a classifier that extracts exactly five tags for the content you receive, 
    you will output only the tags as a comma separated list,
    you will treat every message as a new chat, forgetting the context of previous messages, 
    if the message is an url of a website, get the website's metadata to use
    as input for the classifier.";

    fn chat_body(message: String) -> ChatBody {
        ChatBody {
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
                    content: message,
                },
            ],
        }
    }

    pub fn query_llm(text: String, config: Config) -> String {
        // let auth = Auth::from_env().unwrap(); // Load API key from environment OPENAI_API_KEY.
        let auth = Auth::new(&config.get::<String>("llm-openai.api_key").unwrap());
        let uri = &config.get::<String>("llm-openai.api_uri").unwrap();
        let openai = OpenAI::new(auth, uri);
        let body = chat_body(text);
        let rs = openai.chat_completion_create(&body);
        let choice = rs.unwrap().choices;
        let message = &choice[0].message.as_ref().unwrap();
        for c in &choice {
            let m = c.message.as_ref().unwrap();
            println!("role: {:?}, content: {:?}", m.role, m.content);
        }
        message.content.clone()
    }
}
