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
STACK_ID = os.getenv("STRATUM_STACK_ID")
ENDPOINT_ID = os.getenv("STRATUM_ENDPOINT_ID")
STRATUM_BRANCH = os.getenv("STRATUM_BRANCH")
REDROCK_API_URL = os.getenv("REDROCK_API_URL")
REDROCK_API_KEY = os.getenv("REDROCK_API_KEY")
#NODE_NET = os.getenv("NODE_NET")

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
        {"name": "REDROCK_API_KEY", "value": REDROCK_API_KEY}#,
 #       {"name": "NODE_NET", "value": NODE_NET}
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
