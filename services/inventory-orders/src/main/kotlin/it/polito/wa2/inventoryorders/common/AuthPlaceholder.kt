package it.polito.wa2.inventoryorders.common

// TODO(keycloak): delete this file when the customer is read from the JWT

/** Header that tells the controllers who the customer is, until Keycloak is integrated. */
const val CUSTOMER_HEADER = "X-Customer-Id"

/** Customer used when the header is missing. */
const val DEFAULT_CUSTOMER_ID = "11111111-1111-1111-1111-111111111111"

/** Header that tells the controllers who the organizer is, until Keycloak is integrated. */
const val ORGANIZER_HEADER = "X-Organizer-Id"

/** Organizer used when the header is missing. */
const val DEFAULT_ORGANIZER_ID = "22222222-2222-2222-2222-222222222222"