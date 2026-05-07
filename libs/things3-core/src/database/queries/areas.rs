use crate::{
    database::ThingsDatabase,
    error::{Result as ThingsResult, ThingsError},
    models::{Area, ThingsId},
};
use chrono::Utc;
use sqlx::Row;
use tracing::{debug, instrument};

impl ThingsDatabase {
    /// Get all areas
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if area data is invalid
    #[instrument]
    pub async fn get_all_areas(&self) -> ThingsResult<Vec<Area>> {
        // Get all areas, not just visible ones (MCP clients may want to see all)
        let rows = sqlx::query(
            r"
            SELECT 
                uuid, title, visible, `index`
             FROM TMArea 
            ORDER BY `index` ASC
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch areas: {e}")))?;

        let mut areas = Vec::new();
        for row in rows {
            let uuid_str: String = row.get("uuid");
            let area = Area {
                uuid: ThingsId::from_trusted(uuid_str),
                title: row.get("title"),
                notes: None,          // Notes not stored in TMArea table
                projects: Vec::new(), // TODO: Load projects separately
                tags: Vec::new(),     // TODO: Load tags separately
                created: Utc::now(),  // Creation date not available in TMArea
                modified: Utc::now(), // Modification date not available in TMArea
            };
            areas.push(area);
        }

        debug!("Fetched {} areas", areas.len());
        Ok(areas)
    }

    /// Get all areas (alias for `get_all_areas` for compatibility)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if area data is invalid
    #[instrument(skip(self))]
    pub async fn get_areas(&self) -> ThingsResult<Vec<Area>> {
        self.get_all_areas().await
    }
}
