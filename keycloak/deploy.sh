#!/bin/bash
set -euo pipefail

ACR_NAME="prodekoregistry"
REGISTRY="$ACR_NAME.azurecr.io"
IMAGE="$REGISTRY/keycloak"
TAG="$(git rev-parse --short HEAD)"
APP_NAME="prodeko-keycloak"
RESOURCE_GROUP="prodeko-rg"

# Ensure App Service managed identity can pull from ACR
PRINCIPAL_ID=$(az webapp identity show --name "$APP_NAME" --resource-group "$RESOURCE_GROUP" --query principalId -o tsv)
ACR_ID=$(az acr show --name "$ACR_NAME" --query id -o tsv)
az role assignment create --assignee "$PRINCIPAL_ID" --role AcrPull --scope "$ACR_ID" 2>/dev/null || true

echo "Building Keycloak image..."
az acr login --name "$ACR_NAME"
docker build -t "$IMAGE:$TAG" -t "$IMAGE:latest" .
docker push "$IMAGE:$TAG"
docker push "$IMAGE:latest"

echo "Restarting App Service..."
az webapp restart --name "$APP_NAME" --resource-group "$RESOURCE_GROUP"

echo "Deployed keycloak:$TAG"
