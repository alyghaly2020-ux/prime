use parking_lot::RwLock;
use tantivy::{
    collector::TopDocs, doc, query::QueryParser, schema::*, Index, IndexReader, IndexWriter,
};

use super::vector;
use super::MemoryEntry;

struct TantivyIndex {
    _index: Index,
    writer: IndexWriter,
    reader: IndexReader,
    text_field: Field,
}

impl std::fmt::Debug for TantivyIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TantivyIndex")
            .field("text_field", &self.text_field)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct RagEngine {
    chunk_size: usize,
    overlap: usize,
    top_k: usize,
    tantivy: RwLock<Option<TantivyIndex>>,
}

impl Default for RagEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RagEngine {
    pub fn new() -> Self {
        Self {
            chunk_size: 512,
            overlap: 64,
            top_k: 5,
            tantivy: RwLock::new(None),
        }
    }

    /// Ensure the Tantivy index is initialized (lazy).
    fn ensure_index(&self) -> anyhow::Result<()> {
        let mut guard = self.tantivy.write();
        if guard.is_some() {
            return Ok(());
        }

        let mut schema_builder = Schema::builder();
        let text_field = schema_builder.add_text_field("content", TEXT);
        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema);
        let writer = index.writer(50_000_000)?;
        let reader = index.reader()?;

        *guard = Some(TantivyIndex {
            _index: index,
            writer,
            reader,
            text_field,
        });

        Ok(())
    }

    pub fn chunk_document(&self, text: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text.len() {
            let end = std::cmp::min(start + self.chunk_size, text.len());
            if end < text.len() {
                // Try to break at a sentence boundary
                let search_start = end.saturating_sub(100);
                if let Some(period_pos) = text[search_start..end].rfind('.') {
                    let break_pos = search_start + period_pos + 1;
                    if break_pos > start {
                        chunks.push(text[start..break_pos].to_string());
                        start = break_pos.saturating_sub(self.overlap);
                        continue;
                    }
                }
            }
            chunks.push(text[start..end].to_string());
            start = end.saturating_sub(self.overlap);
        }

        chunks
    }

    /// Keyword-based retrieval with TF-IDF-style overlap scoring.
    pub async fn retrieve(&self, query: &str, entries: &[MemoryEntry]) -> Vec<(MemoryEntry, f32)> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        if query_words.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(MemoryEntry, f32)> = entries
            .iter()
            .map(|entry| {
                let content_lower = entry.content.to_lowercase();
                let matches = query_words
                    .iter()
                    .filter(|w| content_lower.contains(*w))
                    .count();
                let score = matches as f32 / query_words.len() as f32;
                (entry.clone(), score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(self.top_k);

        scored
    }

    /// Hybrid retrieval: combine keyword + vector similarity scores.
    pub async fn retrieve_hybrid(
        &self,
        query: &str,
        entries: &[MemoryEntry],
    ) -> Vec<(MemoryEntry, f32)> {
        if entries.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let query_embed = vector::compute_text_embedding(query, 384);

        let mut scored: Vec<(MemoryEntry, f32)> = entries
            .iter()
            .map(|entry| {
                // Keyword score
                let content_lower = entry.content.to_lowercase();
                let kw_matches = query_words
                    .iter()
                    .filter(|w| content_lower.contains(*w))
                    .count();
                let kw_score = if query_words.is_empty() {
                    0.0
                } else {
                    kw_matches as f32 / query_words.len() as f32
                };

                // Vector score
                let emb = entry.embedding.as_ref().unwrap_or(&query_embed);
                let vec_score = vector::cosine_similarity(&query_embed, emb);

                // Combined: 40% keyword, 60% vector
                let combined = 0.4 * kw_score + 0.6 * vec_score;
                (entry.clone(), combined)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(self.top_k);

        scored
    }

    /// Retrieve chunks plus surrounding context from the results list.
    pub async fn retrieve_with_context(
        &self,
        query: &str,
        entries: &[MemoryEntry],
    ) -> Vec<(MemoryEntry, f32)> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        if query_words.is_empty() {
            return Vec::new();
        }

        // Score all entries
        let mut scored: Vec<(MemoryEntry, f32)> = entries
            .iter()
            .map(|entry| {
                let content_lower = entry.content.to_lowercase();
                let matches = query_words
                    .iter()
                    .filter(|w| content_lower.contains(*w))
                    .count();
                let score = matches as f32 / query_words.len() as f32;
                (entry.clone(), score)
            })
            .collect();

        // Sort by score descending, take top_k * 3 for context window
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(self.top_k * 3);

        // Sort by creation time to group context
        scored.sort_by_key(|a| a.0.created_at);

        let mut contextualized: Vec<(MemoryEntry, f32)> = Vec::new();
        for (entry, score) in &scored {
            if let Some(pos) = entries.iter().position(|e| e.id == entry.id) {
                if pos > 0 {
                    let prev = entries[pos - 1].clone();
                    if !contextualized.iter().any(|(e, _)| e.id == prev.id) {
                        contextualized.push((prev, *score * 0.8));
                    }
                }
                if !contextualized.iter().any(|(e, _)| e.id == entry.id) {
                    contextualized.push((entry.clone(), *score));
                }
                if pos + 1 < entries.len() {
                    let next = entries[pos + 1].clone();
                    if !contextualized.iter().any(|(e, _)| e.id == next.id) {
                        contextualized.push((next, *score * 0.8));
                    }
                }
            }
        }

        contextualized.truncate(self.top_k);
        contextualized
    }

    /// Rerank results by relevance using embedding similarity.
    pub async fn rerank(
        &self,
        query: &str,
        results: &[(MemoryEntry, f32)],
    ) -> Vec<(MemoryEntry, f32)> {
        if results.is_empty() {
            return Vec::new();
        }

        let query_embed = vector::compute_text_embedding(query, 384);

        let mut reranked: Vec<(MemoryEntry, f32)> = results
            .iter()
            .map(|(entry, _old_score)| {
                let emb = entry
                    .embedding.clone()
                    .unwrap_or_else(|| vector::compute_text_embedding(&entry.content, 384));
                let score = vector::cosine_similarity(&query_embed, &emb);
                (entry.clone(), score)
            })
            .collect();

        reranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        reranked
    }

    /// Store a document in the Tantivy index for full-text search.
    pub fn add_document(&self, text: &str) -> anyhow::Result<()> {
        self.ensure_index()?;

        let mut guard = self.tantivy.write();
        if let Some(ref mut idx) = &mut *guard {
            idx.writer.add_document(doc!(idx.text_field => text))?;
            idx.writer.commit()?;
            idx.reader.reload()?;
        }

        Ok(())
    }

    /// Search the Tantivy full-text index.
    pub fn search_index(&self, query: &str) -> anyhow::Result<Vec<String>> {
        self.ensure_index()?;

        let guard = self.tantivy.read();
        if let Some(ref idx) = &*guard {
            let searcher = idx.reader.searcher();
            let qp = QueryParser::for_index(&idx._index, vec![idx.text_field]);
            let query = qp.parse_query(query)?;
            let top_docs = searcher.search(&query, &TopDocs::with_limit(self.top_k))?;

            let mut results = Vec::new();
            for (_score, doc_addr) in top_docs {
                let doc = searcher.doc::<tantivy::TantivyDocument>(doc_addr)?;
                if let Some(text) = doc.get_first(idx.text_field).and_then(|v| v.as_str()) {
                    results.push(text.to_string());
                }
            }
            return Ok(results);
        }

        Ok(Vec::new())
    }

    pub fn set_top_k(&mut self, k: usize) {
        self.top_k = k;
    }
}
