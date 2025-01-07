-- 7 января 2025
--
ALTER TABLE blocks
ADD COLUMN is_rewards_calculated UInt8 DEFAULT 0;

--
--
ALTER TABLE shares ADD COLUMN account_name String;
ALTER TABLE blocks ADD COLUMN account_name String;

--
-- 
ALTER TABLE shares DROP COLUMN user_identity;
ALTER TABLE blocks DROP COLUMN user_identity;