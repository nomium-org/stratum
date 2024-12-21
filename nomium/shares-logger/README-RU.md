Это для Андрея преимущественно :-)

1. Да, можно пересоздать MV и заполнить его данными из источника. Ключевое слово: POPULATE (уже добавил в nomium/shares-logger/src/storage/clickhouse/queries/hashrate_view.sql)

2. Инструкция как это сделать из консоли, данные доступа и пути к файлам поменять на свои.

Проверка существования представления:
```bash
echo "SHOW TABLES FROM mining LIKE 'mv_hash_rate%'" | curl 'http://localhost:8123/?database=mining' --data-binary @- -u default:5555
```

Удаление существующего представления:
```bash
echo "DROP TABLE IF EXISTS mining.mv_hash_rate_stats" | curl 'http://localhost:8123/?database=mining' --data-binary @- -u default:5555
```

Создание нового представления из файла:
```bash
curl 'http://localhost:8123/?user=default&password=5555&database=mining' --data-binary @/home/ro/projects/nomium/dev/stratum/nomium/shares-logger/src/storage/clickhouse/queries/hashrate_view.sql
```

Проверка что представление создалось:
```bash
echo "SELECT name, engine FROM system.tables WHERE database = 'mining' AND name = 'mv_hash_rate_stats'" | curl 'http://localhost:8123/?database=mining' --data-binary @- -u default:5555
```

3. Базовые запросы к MV:

```bash
curl -X POST 'http://localhost:8123/' -H "X-ClickHouse-User: default" -H "X-ClickHouse-Key: 5555" -d "
SELECT
    worker_id,
    sum(total_hashes) / (24 * 60 * 60) AS hash_rate
FROM mining.mv_hash_rate_stats
WHERE period_start >= toStartOfMinute(now() - INTERVAL 1 DAY)
GROUP BY worker_id
FORMAT Pretty"
```

```bash
curl -X POST 'http://localhost:8123/' -H "X-ClickHouse-User: default" -H "X-ClickHouse-Key: 5555" -d "
SELECT
    worker_id,
    sum(total_hashes) / (10 * 60) AS hash_rate
FROM mining.mv_hash_rate_stats
WHERE period_start >= toStartOfMinute(now() - INTERVAL 10 MINUTE)
GROUP BY worker_id
FORMAT Pretty"
```

4. Само MV радикально почикал :) Должно работать быстро. Если поймем что там чего-то нам не хватает - добавим. 