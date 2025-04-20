import random
import os
from urllib.parse import urlparse
import webbrowser
import secrets
from flask import Flask, request, redirect
import psycopg2
from dotenv import load_dotenv
from requests.auth import HTTPBasicAuth
import requests
from psycopg2.errors import UniqueViolation

load_dotenv()

# Configuration
oauth2_config = {
    'client_id': os.getenv('OAUTH_CLIENT_ID'),
    'client_secret': os.getenv('OAUTH_CLIENT_SECRET'),
    'redirect_uri': 'http://127.0.0.1:8080/auth/callback',
    'scope': 'openid profile email',
    'auth_url': os.getenv('OAUTH_ISSUER_URL') + '/oauth2/auth',
    'token_url': os.getenv('OAUTH_ISSUER_URL') + '/oauth2/token',
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

roles = [
    {'name': "prodeko-external-member", 'color': "#ffffff",
        'description': "Prodeko external member"},
    {'name': "prodeko-full-member", 'color': "#fffff0",
        'description': "Prodeko external member"},
    {'name': "prodeko-alumni", 'color': "#ffff0f",
        'description': "Prodeko external member"},
    {'name': "pora-member", 'color': "#fff0ff",
        'description': "Prodeko external member"},
    {'name': "root-users", 'color': "#ff0fff",
        'description': "Prodeko external member"},
    {'name': "prodeko-board", 'color': "#f0ffff",
        'description': "Prodeko external member"},
    {'name': "prodeko-official", 'color': "#ff0fff",
        'description': "Prodeko external member"},
    {'name': "prodeko-webbitiimi", 'color': "#0fffff",
        'description': "Prodeko external member"}
]

role_member_date_choices = [
    ('2021-01-01', '2022-01-01'),
    ('2022-01-01', '2023-01-01'),
    ('2023-01-01', '2024-01-01'),
    ('2024-01-01', '2025-01-01')
]

targetable_roles = [
    "prodeko-external-member",
    "prodeko-full-member",
    "prodeko-alumni",
    "pora-member"
]

app = Flask(__name__)

# Global variable to store tokens
tokens = {}


@app.route('/')
def home():
    return "Welcome to the Fake Data Generator. Use /login to start the OAuth flow."


@app.route('/login', methods=['GET'])
def login():
    # Generate a secure state parameter
    state = secrets.token_urlsafe(16)
    tokens['state'] = state  # Store state to validate in callback

    # Redirect user to the OAuth provider for login
    auth_url = (
        f"{oauth2_config['auth_url']}?response_type=code&client_id={oauth2_config['client_id']}"
        f"&redirect_uri={oauth2_config['redirect_uri']}&scope={oauth2_config['scope']}&state={state}"
    )

    return redirect(auth_url)


@app.route('/auth/callback', methods=['GET'])
def auth_callback():
    # Validate state parameter
    state = request.args.get('state')
    if state != tokens.get('state'):
        return "Invalid or missing state parameter.", 400

    # Capture token
    code = request.args.get('code')
    if not code:
        return "Missing access code in the response.", 400

    data = {
        'grant_type': 'authorization_code',
        'code': code,
        'redirect_uri': oauth2_config['redirect_uri'],
    }
    response = requests.post(
        oauth2_config['token_url'],
        data=data,
        auth=HTTPBasicAuth(
            oauth2_config['client_id'], oauth2_config['client_secret']),
        timeout=10
    )
    response.raise_for_status()
    tokens['access_token'] = response.json()['access_token']
    print("Access token is: ", tokens['access_token'])
    return redirect('/generate-data')


def fetch_identities(token):
    api_url = f"{os.getenv('ORY_BASE_URL')}/admin/identities"
    print("Token is: ", token)
    headers = {'Authorization': f'Bearer {token}'}

    response = requests.get(api_url, headers=headers, timeout=10)
    response.raise_for_status()
    print(response.json())
    return response.json()


@app.route('/generate-data', methods=['GET'])
def generate_data():
    token = tokens.get('access_token')
    if not token:
        return redirect('/login')

    # Fetch identities
    identities = fetch_identities(token)

    # Generate fake data
    random_municipality = random.choice(
        ["Espoo", "Helsinki", "Vantaa", "Kauniainen"])

    conn = psycopg2.connect(**db_config)
    cursor = conn.cursor()

    # Truncate tables
    cursor.execute(
        "TRUNCATE TABLE Application, ApplicationTargetableRole, Role, RoleMember, Member CASCADE;")
    
    # Insert Members
    for user in identities:
        cursor.execute(
            """
            INSERT INTO Member (user_id, email, first_name, last_name, home_municipality, has_accepted_policies) 
            VALUES (%s, %s, %s, %s, %s, %s);
            """,
            (user['id'], user['traits']['email'], user['traits']['name']
             ['first'], user['traits']['name']['last'], random_municipality, True)
        )

    # Insert Roles
    for role in roles:
        cursor.execute(
            "INSERT INTO Role (name, color, description) VALUES (%s, %s, %s);",
            (role['name'], role['color'], role['description'])
        )

    # Insert RoleMembers
    for user in identities:
        for _ in range(random.randint(0, 4)):
            date_choice = random.choice(role_member_date_choices)
            role = random.choice(roles)
            try:
                cursor.execute(
                    """
                    INSERT INTO RoleMember (user_id, role_name, valid_from, valid_until)
                    VALUES (%s, %s, %s, %s);
                    """,
                    (user['id'], role['name'], date_choice[0], date_choice[1])
                )
            except UniqueViolation:
                # Skip if the RoleMember already exists
                pass

    # Insert ApplicationTargetableRoles
    for role in targetable_roles:
        cursor.execute(
            """
            INSERT INTO ApplicationTargetableRole (role_name, valid_until, active, payment_link, optional_roles) 
            VALUES (%s, %s, %s, %s, %s);
            """,
            (role, '2025-01-01', True, '', '{"pora-member"}')
        )

    conn.commit()
    cursor.close()
    conn.close()

    return "Fake data generation completed and inserted into the database!"


if __name__ == "__main__":
    # Automatically open the browser for the login endpoint
    LOGIN_URL = "http://127.0.0.1:8080/login"
    webbrowser.open(LOGIN_URL)

    app.run(debug=True, port=8080)
