# Инструкция как из консоли Linux пересоздать MV и заполнить его данными из источника

1. Добавить в MV ключевое слово: POPULATE (уже добавил в nomium/shares-logger/src/storage/clickhouse/queries/hashrate_view.sql)

2. Данные доступа и пути к файлам поменять на свои:

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

// в связи работами над уточнением хэшрейта запросы и структура MV сильно изменятся, ожидаем.  