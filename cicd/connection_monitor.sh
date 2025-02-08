#!/bin/bash

while true; do
    clear
    echo "=== Анализ соединений $(date) ==="
    
    # Количество соединений в разных состояниях
    echo "Статистика по состояниям:"
    ss -ant | awk '{print $1}' | sort | uniq -c
    
    # Подсчет неактивных соединений
    echo -e "\nНеактивные соединения (>60 сек): "
    ss -ant -o | awk '$1=="ESTAB" && $2=="0" && $3=="0" && $6 ~ /timer:\(keepalive/ {count++} END {print count}'
    
    # Проверка лимитов
    echo -e "\nДиапазон открытых портов:"
    cat /proc/sys/net/ipv4/ip_local_port_range
    echo -e "\nСколько портов в использовании:"
    netstat -ant | wc -l

    # Компактная статистика порта 34255
    echo -e "\nПорт 34255:"
    echo -n "Всего соединений: "
    ss -ant | awk '$4 ~ /:34255/ || $5 ~ /:34255/ {count++} END {print count}'
    
    echo -n "Активных ESTABLISHED: "
    ss -ant | awk '$1=="ESTAB" && ($4 ~ /:34255/ || $5 ~ /:34255/) {count++} END {print count}'

        # Проблемные метрики порта 34255
    echo -e "\nПорт 34255 - проблемные метрики:"
    
    echo -n "Переполнение очереди отправки (Send-Q > 0): "
    ss -ant | awk '$3 > 0 && ($4 ~ /:34255/ || $5 ~ /:34255/) {count++} END {print count}'
    
    echo -n "Переполнение очереди приема (Recv-Q > 0): "
    ss -ant | awk '$2 > 0 && ($4 ~ /:34255/ || $5 ~ /:34255/) {count++} END {print count}'
    
    echo -n "Зависшие CLOSE_WAIT: "
    ss -ant | awk '$1=="CLOSE-WAIT" && ($4 ~ /:34255/ || $5 ~ /:34255/) {count++} END {print count}'
    
    echo -n "Долгие TIME_WAIT: "
    ss -ant | awk '$1=="TIME-WAIT" && ($4 ~ /:34255/ || $5 ~ /:34255/) {count++} END {print count}'
    
    echo -n "Незавершенные SYN_RECV: "
    ss -ant | awk '$1=="SYN-RECV" && ($4 ~ /:34255/ || $5 ~ /:34255/) {count++} END {print count}'

    echo -n "Retrans соединения: "
    ss -anti | awk '($4 ~ /:34255/ || $5 ~ /:34255/) && $7 ~ /retrans:/ {count++} END {print count}'
    
    sleep 5
done
