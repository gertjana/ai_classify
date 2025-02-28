#[path = "llm/openai.rs"]
mod openai;

pub mod llm {
    use config::Config;

    use super::openai::openai::LLMOpenAI;

    pub trait LLM {
        fn query(&self, text: String) -> anyhow::Result<String>;
    }

    pub fn query_llm(text: String, config: &Config) -> anyhow::Result<String> {
        let llm_type = config.get::<String>("general.llm-type").unwrap();
        let llm: Box<dyn LLM> = match llm_type.as_str() {
            "openai" => Box::new(LLMOpenAI::new(
                config.get::<String>("llm-openai.api_key").unwrap(),
                config.get::<String>("llm-openai.api_uri").unwrap(),
            )),
            _ => panic!("Unsupported LLM type: {}", llm_type),
        };
        llm.query(text)
    }
}
