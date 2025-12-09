use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDocument {
    pub entity_id: String,
    pub space_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_global_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_space_score: Option<f64>,
    pub indexed_at: String,
}

const SAMPLE_WORDS: &[&str] = &[
    "entity",
    "document",
    "knowledge",
    "graph",
    "space",
    "node",
    "relation",
    "property",
    "value",
    "system",
    "data",
    "information",
    "structure",
    "content",
    "metadata",
    "index",
    "search",
    "query",
    "result",
    "analysis",
    "processing",
    "storage",
    "retrieval",
    "database",
    "collection",
    "record",
    "entry",
    "item",
    "object",
    "instance",
    "block",
];

const ADJECTIVES: &[&str] = &[
    "important",
    "significant",
    "relevant",
    "useful",
    "valuable",
    "critical",
    "essential",
    "primary",
    "secondary",
    "advanced",
    "complex",
    "simple",
    "detailed",
    "comprehensive",
    "extensive",
];

// Misspelled versions of common words to test typo tolerance/fuzziness
// Format: (misspelling, correct_word)
const MISSPELLED_WORDS: &[(&str, &str)] = &[
    ("cbloc", "block"),
    ("knoledge", "knowledge"),
    ("documant", "document"),
    ("entety", "entity"),
    ("grapgh", "graph"),
    ("spase", "space"),
    ("nod", "node"),
    ("relashun", "relation"),
    ("proprty", "property"),
    ("valyu", "value"),
    ("sistem", "system"),
    ("infomation", "information"),
    ("structur", "structure"),
    ("contant", "content"),
    ("metadta", "metadata"),
    ("indx", "index"),
    ("serch", "search"),
    ("quary", "query"),
    ("reslt", "result"),
    ("analisis", "analysis"),
    ("procesing", "processing"),
    ("storag", "storage"),
    ("retrievl", "retrieval"),
    ("databse", "database"),
    ("collecion", "collection"),
    ("recrd", "record"),
    ("entri", "entry"),
    ("objet", "object"),
    ("instanc", "instance"),
];

fn random_word() -> &'static str {
    SAMPLE_WORDS[rand::random::<usize>() % SAMPLE_WORDS.len()]
}

fn random_adjective() -> &'static str {
    ADJECTIVES[rand::random::<usize>() % ADJECTIVES.len()]
}

fn random_sentence(min_words: usize, max_words: usize) -> String {
    let word_count = min_words + (rand::random::<usize>() % (max_words - min_words + 1));
    let mut words = Vec::new();

    for i in 0..word_count {
        let word = random_word();
        if i == 0 {
            words.push(format!("{}{}", &word[..1].to_uppercase(), &word[1..]));
        } else {
            words.push(word.to_string());
        }
    }

    words.join(" ") + "."
}

fn generate_name() -> String {
    let pattern_idx = rand::random::<usize>() % 3;
    match pattern_idx {
        0 => {
            let adj = random_adjective();
            let word = random_word();
            format!(
                "{}{} {}{}",
                &adj[..1].to_uppercase(),
                &adj[1..],
                &word[..1].to_uppercase(),
                &word[1..]
            )
        }
        1 => {
            let word1 = random_word();
            let word2 = random_word();
            format!("{}{} {}", &word1[..1].to_uppercase(), &word1[1..], word2)
        }
        _ => format!("The {} {}", random_adjective(), random_word()),
    }
}

fn generate_description() -> String {
    let sentences = 2 + (rand::random::<usize>() % 3); // 2-4 sentences
    let mut desc_parts = Vec::new();

    for _ in 0..sentences {
        desc_parts.push(random_sentence(8, 20));
    }

    desc_parts.join(" ")
}

pub fn generate_document(space_id: Option<&str>) -> EntityDocument {
    let has_name = rand::random::<f64>() > 0.1; // 90% have names
    let has_description = rand::random::<f64>() > 0.2; // 80% have descriptions
    let has_avatar = rand::random::<f64>() > 0.7; // 30% have avatars
    let has_cover = rand::random::<f64>() > 0.8; // 20% have covers

    EntityDocument {
        entity_id: Uuid::new_v4().to_string(),
        space_id: space_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        name: if has_name {
            Some(generate_name())
        } else {
            None
        },
        description: if has_description {
            Some(generate_description())
        } else {
            None
        },
        avatar: if has_avatar {
            Some(format!(
                "https://example.com/avatars/{}.jpg",
                Uuid::new_v4()
            ))
        } else {
            None
        },
        cover: if has_cover {
            Some(format!("https://example.com/covers/{}.jpg", Uuid::new_v4()))
        } else {
            None
        },
        entity_global_score: if rand::random::<f64>() > 0.5 {
            Some(rand::random::<f64>() * 100.0)
        } else {
            None
        },
        space_score: if rand::random::<f64>() > 0.5 {
            Some(rand::random::<f64>() * 100.0)
        } else {
            None
        },
        entity_space_score: if rand::random::<f64>() > 0.5 {
            Some(rand::random::<f64>() * 100.0)
        } else {
            None
        },
        indexed_at: chrono::Utc::now().to_rfc3339(),
    }
}

pub fn generate_documents(count: usize, space_id: Option<&str>) -> Vec<EntityDocument> {
    (0..count).map(|_| generate_document(space_id)).collect()
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub query: String,
    pub scope: String,
    pub space_id: Option<String>,
    pub limit: usize,
}

const SCOPES: &[&str] = &["GLOBAL", "GLOBAL_BY_SPACE_SCORE", "SPACE_SINGLE", "SPACE"];
const LIMITS: &[usize] = &[10, 20, 50, 100];

fn extract_words(documents: &[EntityDocument]) -> Vec<String> {
    let mut words = std::collections::HashSet::new();

    for doc in documents {
        if let Some(ref name) = doc.name {
            for word in name.to_lowercase().split_whitespace() {
                if word.len() > 2 {
                    words.insert(word.to_string());
                }
            }
        }
        if let Some(ref description) = doc.description {
            for word in description.to_lowercase().split_whitespace() {
                if word.len() > 2 {
                    words.insert(word.to_string());
                }
            }
        }
    }

    words.into_iter().collect()
}

fn generate_word_prefix(word: &str) -> String {
    // Generate a prefix of 2-4 characters for autocomplete testing
    let prefix_len = if word.len() <= 2 {
        word.len()
    } else if word.len() <= 4 {
        2 + rand::random::<usize>() % 2 // 2-3 chars
    } else {
        3 + rand::random::<usize>() % 2 // 3-4 chars
    };
    word.chars().take(prefix_len.min(word.len())).collect()
}

fn generate_query_from_words(words: &[String]) -> String {
    if words.is_empty() {
        let fallback = vec![
            "entity",
            "document",
            "knowledge",
            "graph",
            "space",
            "search",
        ];
        return fallback[rand::random::<usize>() % fallback.len()].to_string();
    }

    let query_type = rand::random::<f64>();

    // Distribution:
    // 50% normal single word
    // 20% word prefix (autocomplete)
    // 15% misspelled word
    // 15% multi-word

    if query_type > 0.5 {
        // 50% normal single word
        words[rand::random::<usize>() % words.len()].clone()
    } else if query_type > 0.3 {
        // 20% word prefix (autocomplete/search-as-you-type)
        let word = &words[rand::random::<usize>() % words.len()];
        generate_word_prefix(word)
    } else if query_type > 0.15 {
        // 15% misspelled word
        let misspelling = MISSPELLED_WORDS[rand::random::<usize>() % MISSPELLED_WORDS.len()];
        misspelling.0.to_string()
    } else {
        // 15% multi-word
        let word_count = (2 + rand::random::<usize>() % 2).min(words.len());
        let mut selected_words = Vec::new();
        let mut available_words = words.to_vec();

        for _ in 0..word_count {
            if available_words.is_empty() {
                break;
            }
            let idx = rand::random::<usize>() % available_words.len();
            selected_words.push(available_words.remove(idx));
        }

        selected_words.join(" ")
    }
}

pub fn generate_query(documents: &[EntityDocument], space_ids: &[String]) -> SearchQuery {
    let words = extract_words(documents);
    let query = generate_query_from_words(&words);
    let scope = SCOPES[rand::random::<usize>() % SCOPES.len()].to_string();
    let limit = LIMITS[rand::random::<usize>() % LIMITS.len()];

    let space_id = if (scope == "SPACE_SINGLE" || scope == "SPACE") && !space_ids.is_empty() {
        Some(space_ids[rand::random::<usize>() % space_ids.len()].clone())
    } else {
        None
    };

    SearchQuery {
        query,
        scope,
        space_id,
        limit,
    }
}
