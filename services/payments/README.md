# Payments

## Conceptual schema

![Payments schema](docs/er-schema-payment.png)

The diagram was drawn with [designER](https://designer.designerdb.io/).
The editable source is [`docs/er-schema-payment.er`](docs/er-schema-payment.er): load it in the tool to change the diagram, then export it again as `docs/er-schema-payment.png`.

## Logical schema

Primary keys in **bold**, `*` = nullable.

- psp_operation(**id**, kind, order_id, status, amount, currency, created_at, failure_reason\*)
- paypal_payment(**operation_id** → psp_operation, expires_at, paypal_order_id\*, payer_action_url\*, paypal_capture_id\*)
- paypal_refund(**operation_id** → psp_operation, paypal_refund_id\*)