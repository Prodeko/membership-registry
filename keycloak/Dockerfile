FROM quay.io/keycloak/keycloak:26.5.4 AS builder
ENV KC_DB=postgres
ENV KC_HEALTH_ENABLED=true
COPY themes/membership /opt/keycloak/themes/membership
COPY realm-export.json /opt/keycloak/data/import/realm-export.json
COPY providers/ /opt/keycloak/providers/
RUN /opt/keycloak/bin/kc.sh build

FROM quay.io/keycloak/keycloak:26.5.4
COPY --from=builder /opt/keycloak/ /opt/keycloak/
ENTRYPOINT ["/opt/keycloak/bin/kc.sh"]
CMD ["start", "--optimized", "--import-realm"]
