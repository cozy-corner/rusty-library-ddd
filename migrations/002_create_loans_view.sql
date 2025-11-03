-- CQRSのRead Model用loans_viewテーブルを作成
CREATE TABLE loans_view (
    loan_id UUID PRIMARY KEY,
    book_id UUID NOT NULL,
    member_id UUID NOT NULL,
    loaned_at TIMESTAMPTZ NOT NULL,
    due_date TIMESTAMPTZ NOT NULL,
    returned_at TIMESTAMPTZ,
    extension_count SMALLINT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT extension_count_check CHECK (extension_count >= 0 AND extension_count <= 1),
    CONSTRAINT status_check CHECK (status IN ('active', 'overdue', 'returned'))
);

-- 会員の貸出中の貸出を検索するインデックス（貸出上限確認用）
CREATE INDEX idx_loans_view_member_active ON loans_view(member_id, status) WHERE status = 'active';

-- 延滞候補を検索するインデックス（バッチ処理用）
CREATE INDEX idx_loans_view_overdue_candidates ON loans_view(status, due_date) WHERE status = 'active';

-- 書籍で検索するインデックス（書籍の貸出状況確認用）
CREATE INDEX idx_loans_view_book_id ON loans_view(book_id);

-- 会員の貸出履歴を検索するインデックス
CREATE INDEX idx_loans_view_member_id ON loans_view(member_id);
