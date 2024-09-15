cat fake-data.sql | docker exec -i auth-membership-postgresd-1 psql -U membership -d membership
