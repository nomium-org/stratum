# RRP-428 Тестирование пропавших шар

#### 1. Добавил логгирование из Proxy, все кладется в единую таблицу (!шары дублируются), вывести количество шар пришедших из Proxy и из Pool так:

```bash
curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "
SELECT 
    user_identity,
    countIf(sequence_number = 1) AS From_Pool,
    countIf(sequence_number = 2) AS From_Proxy
FROM mining.shares
GROUP BY user_identity
ORDER BY (From_Pool + From_Proxy) DESC
LIMIT 10
FORMAT Pretty"
```

За последние 10 минут:

```bash
curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "
SELECT 
    user_identity,
    countIf(sequence_number = 1) AS From_Pool,
    countIf(sequence_number = 2) AS From_Proxy
FROM mining.shares
WHERE timestamp BETWEEN 
    now() - INTERVAL 10 MINUTE 
    AND 
    now()
GROUP BY user_identity
ORDER BY (From_Pool + From_Proxy) DESC
LIMIT 10
FORMAT Pretty"
```