CREATE TABLE subject_requests (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    request_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    details TEXT,
    approved_by TEXT,
    approved_at TIMESTAMP WITH TIME ZONE,
    rejected_by TEXT,
    rejected_at TIMESTAMP WITH TIME ZONE,
    cancelled_at TIMESTAMP WITH TIME ZONE,
    decision_comment TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (approved_by) REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (rejected_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_subject_requests_user_id ON subject_requests(user_id);
CREATE INDEX idx_subject_requests_status ON subject_requests(status);
CREATE INDEX idx_subject_requests_request_type ON subject_requests(request_type);
CREATE INDEX idx_subject_requests_created_at ON subject_requests(created_at);
