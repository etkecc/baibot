
CREATE TABLE `answers` (
    `id` INTEGER PRIMARY KEY AUTOINCREMENT,
    `bot_id` VARCHAR NOT NULL, 
    `stored_at` DATE NOT NULL,
    `content_length` INTEGER NOT NULL
);
