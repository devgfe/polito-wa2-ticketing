package it.polito.wa2.inventoryorders.config

import org.springframework.context.annotation.Bean
import org.springframework.context.annotation.Configuration
import org.springframework.http.client.SimpleClientHttpRequestFactory
import org.springframework.web.client.RestClient
import java.time.Clock

@Configuration
class WiringConfig {

    /** Injectable time: the services never call Instant.now() themselves; tests pass a fixed clock. */
    @Bean
    fun clock(): Clock = Clock.systemUTC()

    /** The client of Payments, for the synchronous call createPayment (ADR-0004). */
    @Bean
    fun paymentsClient(props: PaymentsProperties): RestClient = RestClient.builder()
        .baseUrl(props.baseUrl)
        .requestFactory(SimpleClientHttpRequestFactory().apply {
            setConnectTimeout(props.timeout)   // time to open the connection
            setReadTimeout(props.timeout)      // time to wait for the answer
        })
        .build()
}