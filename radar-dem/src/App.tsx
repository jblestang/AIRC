import { useEffect, useRef, useState } from 'react'
import './App.css'

import maplibregl from 'maplibre-gl'
import type { Map as MapLibreMap, RasterDEMSourceSpecification, HillshadeLayerSpecification } from 'maplibre-gl'
import 'maplibre-gl/dist/maplibre-gl.css'

function App() {
	const mapContainerRef = useRef<HTMLDivElement | null>(null)
	const mapRef = useRef<MapLibreMap | null>(null)
	const [exaggeration, setExaggeration] = useState<number>(1.5)

	useEffect(() => {
		if (!mapContainerRef.current || mapRef.current) return

		const map = new maplibregl.Map({
			container: mapContainerRef.current,
			style: {
				version: 8,
				sources: {
					'carto': {
						type: 'raster',
						tiles: [
							'https://a.basemaps.cartocdn.com/rastertiles/light_all/{z}/{x}/{y}@2x.png',
						],
						tileSize: 256,
						attribution:
							"© OpenStreetMap contributors © CARTO",
					},
					'dem': {
						type: 'raster-dem',
						tiles: [
							'https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png',
						],
						tileSize: 256,
						encoding: 'terrarium',
					} as RasterDEMSourceSpecification,
				},
				layers: [
					{ id: 'carto', type: 'raster', source: 'carto' },
				],
			},
			center: [7.4474, 46.9470],
			zoom: 8,
			pitch: 60,
			bearing: 20,
		})

		map.on('style.load', () => {
			map.setTerrain({ source: 'dem', exaggeration })

			const hillshadeLayer: HillshadeLayerSpecification = {
				id: 'hillshade',
				type: 'hillshade',
				source: 'dem',
				layout: { visibility: 'visible' },
				paint: {
					'hillshade-exaggeration': 0.5,
				},
			}
			map.addLayer(hillshadeLayer)
		})

		mapRef.current = map

		return () => {
			map.remove()
			mapRef.current = null
		}
	}, [])

	useEffect(() => {
		if (!mapRef.current) return
		mapRef.current.setTerrain({ source: 'dem', exaggeration })
	}, [exaggeration])

	return (
		<div style={{ height: '100%', width: '100%', position: 'relative' }}>
			<div ref={mapContainerRef} style={{ position: 'absolute', inset: 0 }} />
			<div style={{ position: 'absolute', top: 12, left: 12, background: 'white', padding: 8, borderRadius: 6, boxShadow: '0 1px 4px rgba(0,0,0,0.2)' }}>
				<label style={{ display: 'block', fontSize: 12, marginBottom: 4 }}>Terrain Exaggeration: {exaggeration.toFixed(1)}x</label>
				<input
					type="range"
					min={0}
					max={5}
					step={0.1}
					value={exaggeration}
					onChange={(e) => setExaggeration(parseFloat(e.target.value))}
				/>
			</div>
		</div>
	)
}

export default App
