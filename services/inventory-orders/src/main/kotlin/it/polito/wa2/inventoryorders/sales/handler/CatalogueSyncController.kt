package it.polito.wa2.inventoryorders.sales.handler

import it.polito.wa2.inventoryorders.sales.service.CatalogueSyncService
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestMapping
import org.springframework.web.bind.annotation.RestController

@RestController
@RequestMapping("/api/v1/internal")
class CatalogueSyncController(private val syncService: CatalogueSyncService) {

    @PostMapping("/sync/events/{eventId}")
    fun syncEvent(@PathVariable eventId: Long): ResponseEntity<Void> {
        syncService.syncEvent(eventId)
        return ResponseEntity.noContent().build()
    }
}
