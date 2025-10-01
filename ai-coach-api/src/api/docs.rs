use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::get,
    Router,
};
use serde_json::json;

/// Get OpenAPI specification
pub async fn openapi_spec() -> Result<Json<serde_json::Value>, StatusCode> {
    let spec = json!({
        "openapi": "3.0.3",
        "info": {
            "title": "AI Coach API",
            "description": "Comprehensive REST API for AI-powered coaching platform",
            "version": "1.0.0",
            "contact": {
                "name": "AI Coach Team",
                "email": "api@ai-coach.com"
            },
            "license": {
                "name": "MIT",
                "url": "https://opensource.org/licenses/MIT"
            }
        },
        "servers": [
            {
                "url": "/api/v1",
                "description": "Version 1 API"
            }
        ],
        "paths": {
            "/auth/login": {
                "post": {
                    "tags": ["Authentication"],
                    "summary": "User login",
                    "description": "Authenticate user and return JWT token",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "email": {
                                            "type": "string",
                                            "format": "email",
                                            "example": "user@example.com"
                                        },
                                        "password": {
                                            "type": "string",
                                            "minLength": 8,
                                            "example": "password123"
                                        }
                                    },
                                    "required": ["email", "password"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Successful authentication",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/AuthResponse"
                                    }
                                }
                            }
                        },
                        "401": {
                            "description": "Invalid credentials",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ApiError"
                                    }
                                }
                            }
                        },
                        "429": {
                            "description": "Rate limit exceeded",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/RateLimitError"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/training/upload": {
                "post": {
                    "tags": ["Training"],
                    "summary": "Upload training file",
                    "description": "Upload and process training data file (TCX, GPX, CSV)",
                    "security": [{"bearerAuth": []}],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "multipart/form-data": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "file": {
                                            "type": "string",
                                            "format": "binary",
                                            "description": "Training data file"
                                        }
                                    },
                                    "required": ["file"]
                                }
                            }
                        }
                    },
                    "parameters": [
                        {
                            "name": "process_immediately",
                            "in": "query",
                            "description": "Process file immediately after upload",
                            "schema": {
                                "type": "boolean",
                                "default": false
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "File uploaded successfully",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/FileUploadResponse"
                                    }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid file or request",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ApiError"
                                    }
                                }
                            }
                        },
                        "401": {
                            "description": "Unauthorized",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ApiError"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/goals": {
                "get": {
                    "tags": ["Goals"],
                    "summary": "Get user goals",
                    "description": "Retrieve all goals for the authenticated user with optional filtering",
                    "security": [{"bearerAuth": []}],
                    "parameters": [
                        {
                            "name": "status",
                            "in": "query",
                            "description": "Filter by goal status",
                            "schema": {
                                "type": "string",
                                "enum": ["active", "completed", "paused", "cancelled"]
                            }
                        },
                        {
                            "name": "goal_type",
                            "in": "query",
                            "description": "Filter by goal type",
                            "schema": {
                                "type": "string",
                                "enum": ["performance", "fitness", "weight", "event", "training", "custom"]
                            }
                        },
                        {
                            "name": "limit",
                            "in": "query",
                            "description": "Maximum number of goals to return",
                            "schema": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 100,
                                "default": 50
                            }
                        },
                        {
                            "name": "offset",
                            "in": "query",
                            "description": "Number of goals to skip",
                            "schema": {
                                "type": "integer",
                                "minimum": 0,
                                "default": 0
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Goals retrieved successfully",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "$ref": "#/components/schemas/Goal"
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "tags": ["Goals"],
                    "summary": "Create new goal",
                    "description": "Create a new goal for the authenticated user",
                    "security": [{"bearerAuth": []}],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/CreateGoalRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Goal created successfully",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/GoalResponse"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/analytics/trends": {
                "get": {
                    "tags": ["Analytics"],
                    "summary": "Get performance trends",
                    "description": "Retrieve performance trends and statistics over time",
                    "security": [{"bearerAuth": []}],
                    "parameters": [
                        {
                            "name": "period",
                            "in": "query",
                            "description": "Analysis period",
                            "schema": {
                                "type": "string",
                                "enum": ["day", "week", "month", "year"],
                                "default": "month"
                            }
                        },
                        {
                            "name": "start_date",
                            "in": "query",
                            "description": "Start date for analysis (ISO date)",
                            "schema": {
                                "type": "string",
                                "format": "date"
                            }
                        },
                        {
                            "name": "end_date",
                            "in": "query",
                            "description": "End date for analysis (ISO date)",
                            "schema": {
                                "type": "string",
                                "format": "date"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Performance trends retrieved successfully",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/PerformanceTrendsResponse"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }
            },
            "schemas": {
                "ApiError": {
                    "type": "object",
                    "properties": {
                        "error_code": {
                            "type": "string",
                            "example": "INVALID_REQUEST"
                        },
                        "message": {
                            "type": "string",
                            "example": "The request contains invalid data"
                        },
                        "details": {
                            "type": "object",
                            "nullable": true
                        }
                    },
                    "required": ["error_code", "message"]
                },
                "RateLimitError": {
                    "type": "object",
                    "properties": {
                        "error_code": {
                            "type": "string",
                            "example": "RATE_LIMIT_EXCEEDED"
                        },
                        "message": {
                            "type": "string",
                            "example": "Too many requests per minute"
                        },
                        "retry_after": {
                            "type": "integer",
                            "example": 60,
                            "description": "Seconds to wait before retrying"
                        }
                    },
                    "required": ["error_code", "message", "retry_after"]
                },
                "AuthResponse": {
                    "type": "object",
                    "properties": {
                        "token": {
                            "type": "string",
                            "description": "JWT authentication token"
                        },
                        "expires_at": {
                            "type": "string",
                            "format": "date-time",
                            "description": "Token expiration time"
                        },
                        "user": {
                            "$ref": "#/components/schemas/User"
                        }
                    },
                    "required": ["token", "expires_at", "user"]
                },
                "User": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "format": "uuid"
                        },
                        "email": {
                            "type": "string",
                            "format": "email"
                        },
                        "name": {
                            "type": "string",
                            "nullable": true
                        },
                        "created_at": {
                            "type": "string",
                            "format": "date-time"
                        }
                    },
                    "required": ["id", "email", "created_at"]
                },
                "Goal": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "format": "uuid"
                        },
                        "user_id": {
                            "type": "string",
                            "format": "uuid"
                        },
                        "title": {
                            "type": "string",
                            "example": "Complete Marathon"
                        },
                        "description": {
                            "type": "string",
                            "example": "Run a marathon in under 4 hours"
                        },
                        "goal_type": {
                            "type": "string",
                            "enum": ["performance", "fitness", "weight", "event", "training", "custom"]
                        },
                        "target_value": {
                            "type": "number",
                            "nullable": true,
                            "example": 240.0
                        },
                        "current_value": {
                            "type": "number",
                            "nullable": true,
                            "example": 270.0
                        },
                        "unit": {
                            "type": "string",
                            "nullable": true,
                            "example": "minutes"
                        },
                        "target_date": {
                            "type": "string",
                            "format": "date",
                            "nullable": true
                        },
                        "status": {
                            "type": "string",
                            "enum": ["active", "completed", "paused", "cancelled"]
                        },
                        "priority": {
                            "type": "string",
                            "enum": ["low", "medium", "high", "critical"]
                        },
                        "created_at": {
                            "type": "string",
                            "format": "date-time"
                        },
                        "updated_at": {
                            "type": "string",
                            "format": "date-time"
                        }
                    },
                    "required": ["id", "user_id", "title", "description", "goal_type", "status", "priority", "created_at", "updated_at"]
                },
                "CreateGoalRequest": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 200,
                            "example": "Complete Marathon"
                        },
                        "description": {
                            "type": "string",
                            "maxLength": 1000,
                            "example": "Run a marathon in under 4 hours"
                        },
                        "goal_type": {
                            "type": "string",
                            "enum": ["performance", "fitness", "weight", "event", "training", "custom"]
                        },
                        "target_value": {
                            "type": "number",
                            "nullable": true
                        },
                        "unit": {
                            "type": "string",
                            "nullable": true
                        },
                        "target_date": {
                            "type": "string",
                            "format": "date",
                            "nullable": true
                        },
                        "priority": {
                            "type": "string",
                            "enum": ["low", "medium", "high", "critical"]
                        }
                    },
                    "required": ["title", "description", "goal_type", "priority"]
                },
                "GoalResponse": {
                    "type": "object",
                    "properties": {
                        "goal": {
                            "$ref": "#/components/schemas/Goal"
                        },
                        "progress_percentage": {
                            "type": "number",
                            "minimum": 0,
                            "maximum": 100,
                            "nullable": true
                        },
                        "days_remaining": {
                            "type": "integer",
                            "nullable": true
                        },
                        "success": {
                            "type": "boolean"
                        }
                    },
                    "required": ["goal", "success"]
                },
                "FileUploadResponse": {
                    "type": "object",
                    "properties": {
                        "file_id": {
                            "type": "string",
                            "description": "Unique identifier for the uploaded file"
                        },
                        "filename": {
                            "type": "string",
                            "description": "Original filename"
                        },
                        "file_path": {
                            "type": "string",
                            "description": "Server path where file is stored"
                        },
                        "processing_status": {
                            "type": "string",
                            "enum": ["uploaded", "processing", "processed", "failed"]
                        },
                        "metrics": {
                            "type": "object",
                            "nullable": true,
                            "description": "Extracted training metrics if processed"
                        },
                        "job_id": {
                            "type": "string",
                            "nullable": true,
                            "description": "Background job ID if processing asynchronously"
                        }
                    },
                    "required": ["file_id", "filename", "file_path", "processing_status"]
                },
                "PerformanceTrendsResponse": {
                    "type": "object",
                    "properties": {
                        "user_id": {
                            "type": "string",
                            "format": "uuid"
                        },
                        "period": {
                            "type": "string"
                        },
                        "trends": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/TrendData"
                            }
                        },
                        "summary_statistics": {
                            "$ref": "#/components/schemas/SummaryStatistics"
                        },
                        "success": {
                            "type": "boolean"
                        }
                    },
                    "required": ["user_id", "period", "trends", "summary_statistics", "success"]
                },
                "TrendData": {
                    "type": "object",
                    "properties": {
                        "date": {
                            "type": "string",
                            "format": "date"
                        },
                        "metric_name": {
                            "type": "string"
                        },
                        "value": {
                            "type": "number"
                        },
                        "change_from_previous": {
                            "type": "number",
                            "nullable": true
                        },
                        "moving_average_7d": {
                            "type": "number",
                            "nullable": true
                        },
                        "moving_average_30d": {
                            "type": "number",
                            "nullable": true
                        }
                    },
                    "required": ["date", "metric_name", "value"]
                },
                "SummaryStatistics": {
                    "type": "object",
                    "properties": {
                        "total_sessions": {
                            "type": "integer",
                            "minimum": 0
                        },
                        "total_duration_hours": {
                            "type": "number",
                            "minimum": 0
                        },
                        "total_distance_km": {
                            "type": "number",
                            "minimum": 0
                        },
                        "average_session_duration_minutes": {
                            "type": "number",
                            "minimum": 0
                        },
                        "average_tss": {
                            "type": "number",
                            "minimum": 0
                        },
                        "total_tss": {
                            "type": "number",
                            "minimum": 0
                        },
                        "average_intensity_factor": {
                            "type": "number",
                            "minimum": 0,
                            "maximum": 2
                        }
                    },
                    "required": ["total_sessions", "total_duration_hours", "total_distance_km", "average_session_duration_minutes", "average_tss", "total_tss", "average_intensity_factor"]
                }
            }
        },
        "tags": [
            {
                "name": "Authentication",
                "description": "User authentication and authorization"
            },
            {
                "name": "Training",
                "description": "Training data upload and analysis"
            },
            {
                "name": "Goals",
                "description": "Goal setting and tracking"
            },
            {
                "name": "Coaching",
                "description": "Training plans and coaching recommendations"
            },
            {
                "name": "Analytics",
                "description": "Performance analytics and insights"
            },
            {
                "name": "User Profile",
                "description": "User profile and preferences management"
            }
        ]
    });

    Ok(Json(spec))
}

/// Serve Swagger UI HTML
pub async fn swagger_ui() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AI Coach API Documentation</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui.css" />
    <style>
        html {
            box-sizing: border-box;
            overflow: -moz-scrollbars-vertical;
            overflow-y: scroll;
        }
        *, *:before, *:after {
            box-sizing: inherit;
        }
        body {
            margin:0;
            background: #fafafa;
        }
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {
            const ui = SwaggerUIBundle({
                url: '/api/v1/docs/openapi.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout",
                validatorUrl: null,
                tryItOutEnabled: true,
                supportedSubmitMethods: ['get', 'post', 'put', 'delete', 'patch'],
                onComplete: function() {
                    console.log('Swagger UI loaded');
                }
            });
        };
    </script>
</body>
</html>
    "#)
}

/// Create documentation routes
pub fn docs_routes() -> Router {
    Router::new()
        .route("/docs", get(swagger_ui))
        .route("/docs/openapi.json", get(openapi_spec))
}