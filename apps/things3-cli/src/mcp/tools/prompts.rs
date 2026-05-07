use crate::mcp::{
    Content, GetPromptRequest, GetPromptResult, McpError, McpResult, ThingsMcpServer,
};
use serde_json::Value;

impl ThingsMcpServer {
    /// Handle prompt request
    pub(in crate::mcp) async fn handle_prompt_request(
        &self,
        request: GetPromptRequest,
    ) -> McpResult<GetPromptResult> {
        let prompt_name = &request.name;
        let arguments = request.arguments.unwrap_or_default();

        match prompt_name.as_str() {
            "task_review" => self.handle_task_review_prompt(arguments).await,
            "project_planning" => self.handle_project_planning_prompt(arguments).await,
            "productivity_analysis" => self.handle_productivity_analysis_prompt(arguments).await,
            "backup_strategy" => self.handle_backup_strategy_prompt(arguments).await,
            _ => Err(McpError::prompt_not_found(prompt_name)),
        }
    }

    /// Handle task review prompt
    pub(in crate::mcp) async fn handle_task_review_prompt(
        &self,
        args: Value,
    ) -> McpResult<GetPromptResult> {
        let task_title = args
            .get("task_title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_parameter("task_title"))?;
        let task_notes = args.get("task_notes").and_then(|v| v.as_str());
        let context = args.get("context").and_then(|v| v.as_str());

        // Get current data for context
        let db = &self.db;
        let inbox_tasks = db
            .get_inbox(Some(5))
            .await
            .map_err(|e| McpError::database_operation_failed("get_inbox for task_review", e))?;
        let today_tasks = db
            .get_today(Some(5))
            .await
            .map_err(|e| McpError::database_operation_failed("get_today for task_review", e))?;
        let _ = db;

        let prompt_text = format!(
            "# Task Review: {}\n\n\
            ## Current Task Details\n\
            - **Title**: {}\n\
            - **Notes**: {}\n\
            - **Context**: {}\n\n\
            ## Review Checklist\n\
            Please review this task for:\n\
            1. **Clarity**: Is the task title clear and actionable?\n\
            2. **Completeness**: Does it have all necessary details?\n\
            3. **Priority**: How urgent/important is this task?\n\
            4. **Dependencies**: Are there any prerequisites?\n\
            5. **Time Estimate**: How long should this take?\n\n\
            ## Current Context\n\
            - **Inbox Tasks**: {} tasks\n\
            - **Today's Tasks**: {} tasks\n\n\
            ## Recommendations\n\
            Based on the current workload and task details, provide specific recommendations for:\n\
            - Improving task clarity\n\
            - Breaking down complex tasks\n\
            - Setting appropriate deadlines\n\
            - Managing dependencies\n\n\
            ## Next Steps\n\
            Suggest concrete next steps to move this task forward effectively.",
            task_title,
            task_title,
            task_notes.unwrap_or("No notes provided"),
            context.unwrap_or("No additional context"),
            inbox_tasks.len(),
            today_tasks.len()
        );

        Ok(GetPromptResult {
            content: vec![Content::Text { text: prompt_text }],
            is_error: false,
        })
    }

    /// Handle project planning prompt
    pub(in crate::mcp) async fn handle_project_planning_prompt(
        &self,
        args: Value,
    ) -> McpResult<GetPromptResult> {
        let project_title = args
            .get("project_title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_parameter("project_title"))?;
        let project_description = args.get("project_description").and_then(|v| v.as_str());
        let deadline = args.get("deadline").and_then(|v| v.as_str());
        let complexity = args
            .get("complexity")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");

        // Get current data for context
        let db = &self.db;
        let projects = db.get_projects(None).await.map_err(|e| {
            McpError::database_operation_failed("get_projects for project_planning", e)
        })?;
        let areas = db.get_areas().await.map_err(|e| {
            McpError::database_operation_failed("get_areas for project_planning", e)
        })?;
        let _ = db;

        let prompt_text = format!(
            "# Project Planning: {}\n\n\
            ## Project Overview\n\
            - **Title**: {}\n\
            - **Description**: {}\n\
            - **Deadline**: {}\n\
            - **Complexity**: {}\n\n\
            ## Planning Framework\n\
            Please help plan this project by:\n\
            1. **Breaking down** the project into manageable tasks\n\
            2. **Estimating** time requirements for each task\n\
            3. **Identifying** dependencies between tasks\n\
            4. **Suggesting** milestones and checkpoints\n\
            5. **Recommending** project organization (areas, tags, etc.)\n\n\
            ## Current Context\n\
            - **Existing Projects**: {} projects\n\
            - **Available Areas**: {} areas\n\n\
            ## Task Breakdown\n\
            Create a detailed task list with:\n\
            - Clear, actionable task titles\n\
            - Estimated time for each task\n\
            - Priority levels\n\
            - Dependencies\n\
            - Suggested deadlines\n\n\
            ## Project Organization\n\
            Suggest:\n\
            - Appropriate area for this project\n\
            - Useful tags for organization\n\
            - Project structure and hierarchy\n\n\
            ## Risk Assessment\n\
            Identify potential challenges and mitigation strategies.\n\n\
            ## Success Metrics\n\
            Define how to measure project success and completion.",
            project_title,
            project_title,
            project_description.unwrap_or("No description provided"),
            deadline.unwrap_or("No deadline specified"),
            complexity,
            projects.len(),
            areas.len()
        );

        Ok(GetPromptResult {
            content: vec![Content::Text { text: prompt_text }],
            is_error: false,
        })
    }

    /// Handle productivity analysis prompt
    pub(in crate::mcp) async fn handle_productivity_analysis_prompt(
        &self,
        args: Value,
    ) -> McpResult<GetPromptResult> {
        let time_period = args
            .get("time_period")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_parameter("time_period"))?;
        let focus_area = args
            .get("focus_area")
            .and_then(|v| v.as_str())
            .unwrap_or("all");
        let include_recommendations = args
            .get("include_recommendations")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        // Get current data for analysis
        let db = &self.db;
        let inbox_tasks = db.get_inbox(None).await.map_err(|e| {
            McpError::database_operation_failed("get_inbox for productivity_analysis", e)
        })?;
        let today_tasks = db.get_today(None).await.map_err(|e| {
            McpError::database_operation_failed("get_today for productivity_analysis", e)
        })?;
        let projects = db.get_projects(None).await.map_err(|e| {
            McpError::database_operation_failed("get_projects for productivity_analysis", e)
        })?;
        let areas = db.get_areas().await.map_err(|e| {
            McpError::database_operation_failed("get_areas for productivity_analysis", e)
        })?;
        let _ = db;

        let completed_tasks = projects
            .iter()
            .filter(|p| p.status == things3_core::TaskStatus::Completed)
            .count();
        let incomplete_tasks = projects
            .iter()
            .filter(|p| p.status == things3_core::TaskStatus::Incomplete)
            .count();

        let prompt_text = format!(
            "# Productivity Analysis - {}\n\n\
            ## Analysis Period: {}\n\
            ## Focus Area: {}\n\n\
            ## Current Data Overview\n\
            - **Inbox Tasks**: {} tasks\n\
            - **Today's Tasks**: {} tasks\n\
            - **Total Projects**: {} projects\n\
            - **Areas**: {} areas\n\
            - **Completed Tasks**: {} tasks\n\
            - **Incomplete Tasks**: {} tasks\n\n\
            ## Analysis Framework\n\
            Please analyze productivity patterns focusing on:\n\n\
            ### 1. Task Completion Patterns\n\
            - Completion rates over the period\n\
            - Task types that are completed vs. delayed\n\
            - Time patterns in task completion\n\n\
            ### 2. Workload Distribution\n\
            - Balance between different areas/projects\n\
            - Task complexity distribution\n\
            - Deadline adherence patterns\n\n\
            ### 3. Time Management\n\
            - Task scheduling effectiveness\n\
            - Inbox vs. scheduled task completion\n\
            - Overdue task patterns\n\n\
            ### 4. Project Progress\n\
            - Project completion rates\n\
            - Project complexity vs. completion time\n\
            - Area-based productivity differences\n\n\
            ## Key Insights\n\
            Identify:\n\
            - Peak productivity times\n\
            - Most/least productive areas\n\
            - Common bottlenecks\n\
            - Success patterns\n\n\
            ## Recommendations\n\
            {}",
            time_period,
            time_period,
            focus_area,
            inbox_tasks.len(),
            today_tasks.len(),
            projects.len(),
            areas.len(),
            completed_tasks,
            incomplete_tasks,
            if include_recommendations {
                "Provide specific, actionable recommendations for:\n\
                - Improving task completion rates\n\
                - Better time management\n\
                - Workload balancing\n\
                - Process optimization\n\
                - Goal setting and tracking"
            } else {
                "Focus on analysis without recommendations"
            }
        );

        Ok(GetPromptResult {
            content: vec![Content::Text { text: prompt_text }],
            is_error: false,
        })
    }

    /// Handle backup strategy prompt
    pub(in crate::mcp) async fn handle_backup_strategy_prompt(
        &self,
        args: Value,
    ) -> McpResult<GetPromptResult> {
        let data_volume = args
            .get("data_volume")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_parameter("data_volume"))?;
        let frequency = args
            .get("frequency")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_parameter("frequency"))?;
        let retention_period = args
            .get("retention_period")
            .and_then(|v| v.as_str())
            .unwrap_or("3_months");
        let storage_preference = args
            .get("storage_preference")
            .and_then(|v| v.as_str())
            .unwrap_or("hybrid");

        // Get current data for context
        let db = &self.db;
        let projects = db.get_projects(None).await.map_err(|e| {
            McpError::database_operation_failed("get_projects for backup_strategy", e)
        })?;
        let areas = db
            .get_areas()
            .await
            .map_err(|e| McpError::database_operation_failed("get_areas for backup_strategy", e))?;
        let _ = db;

        let prompt_text = format!(
            "# Backup Strategy Recommendation\n\n\
            ## Requirements\n\
            - **Data Volume**: {}\n\
            - **Backup Frequency**: {}\n\
            - **Retention Period**: {}\n\
            - **Storage Preference**: {}\n\n\
            ## Current Data Context\n\
            - **Projects**: {} projects\n\
            - **Areas**: {} areas\n\
            - **Database Type**: SQLite (Things 3)\n\n\
            ## Backup Strategy Analysis\n\n\
            ### 1. Data Assessment\n\
            Analyze the current data volume and growth patterns:\n\
            - Database size estimation\n\
            - Growth rate projections\n\
            - Critical data identification\n\n\
            ### 2. Backup Frequency Optimization\n\
            For {} frequency backups:\n\
            - Optimal timing considerations\n\
            - Incremental vs. full backup strategy\n\
            - Performance impact analysis\n\n\
            ### 3. Storage Strategy\n\
            For {} storage preference:\n\
            - Local storage recommendations\n\
            - Cloud storage options\n\
            - Hybrid approach benefits\n\
            - Cost considerations\n\n\
            ### 4. Retention Policy\n\
            For {} retention period:\n\
            - Data lifecycle management\n\
            - Compliance considerations\n\
            - Storage optimization\n\n\
            ## Recommended Implementation\n\
            Provide specific recommendations for:\n\
            - Backup tools and software\n\
            - Storage locations and providers\n\
            - Automation setup\n\
            - Monitoring and alerting\n\
            - Recovery procedures\n\n\
            ## Risk Mitigation\n\
            Address:\n\
            - Data loss prevention\n\
            - Backup verification\n\
            - Disaster recovery planning\n\
            - Security considerations\n\n\
            ## Cost Analysis\n\
            Estimate costs for:\n\
            - Storage requirements\n\
            - Backup software/tools\n\
            - Cloud services\n\
            - Maintenance overhead",
            data_volume,
            frequency,
            retention_period,
            storage_preference,
            projects.len(),
            areas.len(),
            frequency,
            storage_preference,
            retention_period
        );

        Ok(GetPromptResult {
            content: vec![Content::Text { text: prompt_text }],
            is_error: false,
        })
    }
}
