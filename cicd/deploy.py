import requests
import json
import os
# Замените эти переменные на ваши значения
PORTAINER_TOKEN = os.getenv("PORTAINER_TOKEN")
NEXUS_URL = os.getenv("NEXUS_URL")
CLICKHOUSE_URL = os.getenv("CLICKHOUSE_URL")
CLICKHOUSE_USER = os.getenv("CLICKHOUSE_USER")
CLICKHOUSE_PASSWORD = os.getenv("CLICKHOUSE_PASSWORD")
PORTAINER_HOST = os.getenv("PORTAINER_HOST")
PORTAINER_PORT = os.getenv("PORTAINER_PORT")
STRATUM_BRANCH = os.getenv("STRATUM_BRANCH")
REDROCK_API_URL = os.getenv("REDROCK_API_URL")
REDROCK_API_KEY = os.getenv("REDROCK_API_KEY")
NODE_CHAIN = os.getenv("NODE_CHAIN")

if NODE_CHAIN == 'pooltest':
    STACK_ID = os.getenv("POOLTEST_STRATUM_STACK_ID")
    ENDPOINT_ID = os.getenv("POOLTEST_STRATUM_PENDPOINT_ID")
    NODE_PORT = os.getenv("POOLTEST_NODE_PORT")
    NODE_KEY = os.getenv("POOLTEST_NODE_KEY")
    COINBASE_TYPE = os.getenv("POOLTEST_COINBASE_TYPE")
    COINBASE_VALUE = os.getenv("POOLTEST_COINBASE_VALUE")
    POOL_PORT = os.getenv("POOLTEST_POOL_PORT")
    TRANSLATOR_PORT = os.getenv("POOLTEST_TRANSLATOR_PORT")
    METRICS_PORT = os.getenv("POOLTEST_METRICS_PORT")
    
elif NODE_CHAIN == 'testnet':
    STACK_ID = os.getenv("TESTNET_STRATUM_STACK_ID")
    ENDPOINT_ID = os.getenv("TESTNET_STRATUM_PENDPOINT_ID")
    NODE_PORT = os.getenv("TESTNET_NODE_PORT")
    NODE_KEY = os.getenv("TESTNET_NODE_KEY")
    COINBASE_TYPE = os.getenv("TESTNET_COINBASE_TYPE")
    COINBASE_VALUE = os.getenv("TESTNET_COINBASE_VALUE")
    POOL_PORT = os.getenv("TESTNET_POOL_PORT")
    TRANSLATOR_PORT = os.getenv("TESTNET_TRANSLATOR_PORT")
    METRICS_PORT = os.getenv("TESTNET_METRICS_PORT")

elif NODE_CHAIN == 'mainnet':
    STACK_ID = os.getenv("MAINNET_STRATUM_STACK_ID")
    ENDPOINT_ID = os.getenv("MAINNET_STRATUM_PENDPOINT_ID")
    NODE_PORT = os.getenv("MAINNET_NODE_PORT")
    NODE_KEY = os.getenv("MAINNET_NODE_KEY")
    COINBASE_TYPE = os.getenv("MAINNET_COINBASE_TYPE")
    COINBASE_VALUE = os.getenv("MAINNET_COINBASE_VALUE")
    POOL_PORT = os.getenv("MAINNET_POOL_PORT")
    TRANSLATOR_PORT = os.getenv("MAINNET_TRANSLATOR_PORT")
    METRICS_PORT = os.getenv("MAINNET_METRICS_PORT")

else:
    print("Variable NODE_CHAIN is not valid")
    exit(1)

url = f"http://{PORTAINER_HOST}:{PORTAINER_PORT}/api/stacks/{STACK_ID}/git/redeploy?endpointId={ENDPOINT_ID}"

headers = {
    "Content-Type": "application/json",
    "X-API-Key": PORTAINER_TOKEN
}

data = {
    "env": [
        {"name": "CLICKHOUSE_URL", "value": CLICKHOUSE_URL},
        {"name": "CLICKHOUSE_USER", "value": CLICKHOUSE_USER},
        {"name": "CLICKHOUSE_PASSWORD", "value": CLICKHOUSE_PASSWORD},
        {"name": "NEXUS_URL", "value": NEXUS_URL},
        {"name": "REDROCK_API_URL", "value": REDROCK_API_URL},
        {"name": "REDROCK_API_KEY", "value": REDROCK_API_KEY},
        {"name": "NODE_CHAIN", "value": NODE_CHAIN},
        {"name": "NODE_PORT", "value": NODE_PORT},
        {"name": "NODE_KEY", "value": NODE_KEY},
        {"name": "COINBASE_TYPE", "value": COINBASE_TYPE},
        {"name": "COINBASE_VALUE", "value": COINBASE_VALUE},
        {"name": "POOL_PORT", "value": POOL_PORT},
        {"name": "TRANSLATOR_PORT", "value": TRANSLATOR_PORT},
        {"name": "METRICS_PORT", "value": METRICS_PORT}
    ],
    "prune": False,
    "pullImage": True,
    "repositoryAuthentication": False,
    "repositoryReferenceName": f"refs/heads/{STRATUM_BRANCH}"
}

response = requests.put(url, headers=headers, data=json.dumps(data))

if response.status_code != 200:
    print(response.text)
    exit(2)
