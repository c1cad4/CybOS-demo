# Planetary Pulse

**Planetary Pulse** is the cybOS public-data dashboard for biodiversity, air quality, and human population indicators.

## Data-integrity rules

- Every metric must show its publisher, reference period, last update, and geographic/species coverage.
- Keep observations, annual statistical estimates, and projections visibly distinct.
- Never extrapolate a global value from one city or one monitoring network.
- Missing data stays missing; no synthetic values presented as real measurements.
- Source links open inside CYBBrowser. External data is not automatically refreshed in this first UI integration.

## Initial wildlife indicator

The 2026 Living Planet Index reports a **73% average decline in monitored wildlife population abundance between 1970 and 2022**, based on 35,803 populations from 5,790 vertebrate species. This is an average index trend, not a claim that 73% of all animals or species have disappeared. The dashboard links to the explanation and source.

Sources:
- https://ourworldindata.org/2026-lpi-update
- https://ourworldindata.org/data-insights/the-living-planet-index-reports-a-73-average-decline-in-studied-wildlife-populations
- https://www.livingplanetindex.org/lpi

## Air quality

The first UI build deliberately reports **NO LIVE FEED** until a provider and location are actually configured. OpenAQ aggregates public sensor measurements but does not cover every monitor worldwide. AirNow's widgets are for supported U.S. locations and must not be presented as a global feed.

Sources:
- https://docs.openaq.org/about/about
- https://docs.openaq.org/resources/latest
- https://www.airnow.gov/aqi-widgets/

## Human population

The current prototype shows the 2025 annual reference values published by Our World in Data using UN World Population Prospects 2024: approximately 8.2 billion people, 132 million births, and 63 million deaths. These are annual estimates/projections, **not a live count**. Automated series ingestion is not yet wired in. Next step: ingest the published population, births and deaths series, retain source metadata, and label any interpolated per-second counters as estimates.

Source:
- https://ourworldindata.org/births-and-deaths

## Next implementation stages

1. Add provider configuration and an asynchronous refresh worker so network requests never block the egui frame.
2. Add OpenAQ v3 with API-key handling, location search, sensor units and measurement timestamps.
3. Ingest OWID/UN annual population, births and deaths series with cached source metadata.
4. Add historical graphs, region filters, stale-data warnings, and exportable source metadata.
5. Complete translations for English, Chinese, Russian and Hindi.
