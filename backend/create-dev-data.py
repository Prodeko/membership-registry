import ory_hydra_client
from ory_hydra_client import ApiClient, Configuration
from ory_hydra_client.api.oauth2_api import OAuth2Api
from ory_hydra_client.model.token_response import TokenResponse
from faker import Faker
import psycopg2
from dotenv import load_dotenv
import os
from urllib.parse import urlparse

# Load environment variables
load_dotenv()

# Configuration
oauth2_config = {
    'client_id': os.getenv('OAUTH_CLIENT_ID'),
    'client_secret': os.getenv('OAUTH_CLIENT_SECRET'),
    'redirect_uri': os.getenv('OAUTH_REDIRECT_URI'),
    'scope': os.getenv('OAUTH_SCOPE'),
    'hydra_url': os.getenv('OAUTH_HYDRA_URL')
}

db_url = os.getenv('DATABASE_URL')
parsed_db_url = urlparse(db_url)
db_config = {
    'dbname': parsed_db_url.path[1:],
    'user': parsed_db_url.username,
    'password': parsed_db_url.password,
    'host': parsed_db_url.hostname,
    'port': parsed_db_url.port
}

def get_oauth2_token():
    configuration = Configuration(
        host=oauth2_config['hydra_url']
    )
    with ApiClient(configuration) as api_client:
        oauth2_api = OAuth2Api(api_client)

        token_response = oauth2_api.oauth2_token(
            grant_type="client_credentials",
            client_id=oauth2_config['client_id'],
            client_secret=oauth2_config['client_secret'],
            scope=oauth2_config['scope']
        )

    return token_response.access_token

def fetch_identities(token):
    api_url = f"{oauth2_config['hydra_url']}/identities"
    headers = {'Authorization': f'Bearer {token}'}

    response = requests.get(api_url, headers=headers)
    response.raise_for_status()

    return response.json()

def generate_fake_data(identities):
    faker = Faker()
    fake_data = []

    for identity in identities:
        fake_entry = {
            'original_id': identity['id'],
            'fake_name': faker.name(),
            'fake_email': faker.email(),
            'fake_phone': faker.phone_number(),
            'fake_address': faker.address(),
        }
        fake_data.append(fake_entry)

    return fake_data

def insert_into_postgres(fake_data):
    conn = psycopg2.connect(**db_config)
    cursor = conn.cursor()

    insert_query = """
    INSERT INTO fake_identities (original_id, fake_name, fake_email, fake_phone, fake_address)
    VALUES (%s, %s, %s, %s, %s)
    """

    for entry in fake_data:
        cursor.execute(insert_query, (
            entry['original_id'],
            entry['fake_name'],
            entry['fake_email'],
            entry['fake_phone'],
            entry['fake_address'],
        ))

    conn.commit()
    cursor.close()
    conn.close()

def main():
    print("Starting the fake data generator...")

    # Step 1: Sign in and get token
    token = get_oauth2_token()

    # Step 2: Fetch identities
    identities = fetch_identities(token)

    # Step 3: Generate fake data
    fake_data = generate_fake_data(identities)

    # Step 4: Insert fake data into PostgreSQL
    insert_into_postgres(fake_data)

    print("Fake data generation completed!")

if __name__ == "__main__":
    main()
