use std::marker::PhantomData;

use router::{
    core::payments,
    db::StorageImpl,
    routes,
    types::{self, api, storage::enums, PaymentAddress},
};

use crate::connector_auth::ConnectorAuthentication;

fn construct_payment_router_data() -> types::PaymentsAuthorizeRouterData {
    let auth = ConnectorAuthentication::new()
        .rapyd
        .expect("Missing Rapyd connector authentication configuration");

    types::RouterData {
        flow: PhantomData,
        merchant_id: "rapyd".to_string(),
        connector: "rapyd".to_string(),
        payment_id: uuid::Uuid::new_v4().to_string(),
        attempt_id: None,
        status: enums::AttemptStatus::default(),
        router_return_url: None,
        auth_type: enums::AuthenticationType::NoThreeDs,
        payment_method: enums::PaymentMethodType::Card,
        connector_auth_type: auth.into(),
        description: Some("This is a test".to_string()),
        return_url: None,
        request: types::PaymentsAuthorizeData {
            amount: 100,
            currency: enums::Currency::USD,
            payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                card_number: "4111111111111111".to_string().into(),
                card_exp_month: "02".to_string().into(),
                card_exp_year: "24".to_string().into(),
                card_holder_name: "John Doe".to_string().into(),
                card_cvc: "123".to_string().into(),
            }),
            confirm: true,
            statement_descriptor_suffix: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            capture_method: None,
            browser_info: None,
            order_details: None,
            email: None,
        },
        response: Err(types::ErrorResponse::default()),
        payment_method_id: None,
        address: PaymentAddress::default(),
        connector_meta_data: None,
        amount_captured: None,
    }
}

fn construct_refund_router_data<F>() -> types::RefundsRouterData<F> {
    let auth = ConnectorAuthentication::new()
        .rapyd
        .expect("Missing Rapyd connector authentication configuration");

    types::RouterData {
        flow: PhantomData,
        connector_meta_data: None,
        merchant_id: "rapyd".to_string(),
        connector: "rapyd".to_string(),
        payment_id: uuid::Uuid::new_v4().to_string(),
        attempt_id: None,
        status: enums::AttemptStatus::default(),
        router_return_url: None,
        payment_method: enums::PaymentMethodType::Card,
        auth_type: enums::AuthenticationType::NoThreeDs,
        connector_auth_type: auth.into(),
        description: Some("This is a test".to_string()),
        return_url: None,
        request: types::RefundsData {
            amount: 100,
            currency: enums::Currency::USD,
            refund_id: uuid::Uuid::new_v4().to_string(),
            connector_transaction_id: String::new(),
            refund_amount: 10,
            connector_metadata: None,
            reason: None,
        },
        response: Err(types::ErrorResponse::default()),
        payment_method_id: None,
        address: PaymentAddress::default(),
        amount_captured: None,
    }
}

#[actix_web::test]
async fn test_rapyd_payment_success() {
    use router::{configs::settings::Settings, connector::Rapyd, services};

    let conf = Settings::new().unwrap();
    static CV: Rapyd = Rapyd;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Rapyd,
        get_token: types::api::GetToken::Connector,
    };
    let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest).await;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        types::api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = construct_payment_router_data();

    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .unwrap();

    println!("{response:?}");

    assert!(
        response.status == enums::AttemptStatus::Charged,
        "The payment failed"
    );
}

#[actix_web::test]
async fn test_rapyd_refund_success() {
    // Successful payment
    use router::{configs::settings::Settings, connector::Rapyd, services};

    let conf = Settings::new().expect("invalid settings");
    let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest).await;
    static CV: Rapyd = Rapyd;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Rapyd,
        get_token: types::api::GetToken::Connector,
    };
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        types::api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let request = construct_payment_router_data();

    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .unwrap();

    println!("{response:?}");

    assert!(
        response.status == enums::AttemptStatus::Charged,
        "The payment failed"
    );
    // Successful refund
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        types::api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();
    let mut refund_request = construct_refund_router_data();

    refund_request.request.connector_transaction_id = match response.response.unwrap() {
        types::PaymentsResponseData::TransactionResponse { resource_id, .. } => {
            resource_id.get_connector_transaction_id().unwrap()
        }
        _ => panic!("Connector transaction id not found"),
    };

    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &refund_request,
        payments::CallConnectorAction::Trigger,
    )
    .await;

    let response = response.unwrap();
    println!("{response:?}");

    assert!(
        response.response.unwrap().refund_status == enums::RefundStatus::Success,
        "The refund failed"
    );
}

#[actix_web::test]
async fn test_rapyd_payment_failure() {
    use router::{configs::settings::Settings, connector::Rapyd, services};

    let conf = Settings::new().expect("invalid settings");
    let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest).await;
    static CV: Rapyd = Rapyd;
    let connector = types::api::ConnectorData {
        connector: Box::new(&CV),
        connector_name: types::Connector::Rapyd,
        get_token: types::api::GetToken::Connector,
    };
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        types::api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();
    let mut request = construct_payment_router_data();
    request.connector_auth_type = types::ConnectorAuthType::BodyKey {
        api_key: "".to_string(),
        key1: "".to_string(),
    };
    let response = services::api::execute_connector_processing_step(
        &state,
        connector_integration,
        &request,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .unwrap();
    assert!(response.response.is_err(), "The payment passed");
}
