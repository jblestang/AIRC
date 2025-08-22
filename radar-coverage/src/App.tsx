import { useEffect, useRef, useState } from 'react'
import maplibregl from 'maplibre-gl'
import 'maplibre-gl/dist/maplibre-gl.css'

function App() {
  const mapContainerRef = useRef<HTMLDivElement | null>(null)
  const mapRef = useRef<maplibregl.Map | null>(null)
  const [isLoaded, setIsLoaded] = useState(false)
  const [terrainEnabled, setTerrainEnabled] = useState(true)
  const [exaggeration, setExaggeration] = useState(1.5)

  useEffect(() => {
    if (!mapContainerRef.current) return

    const map = new maplibregl.Map({
      container: mapContainerRef.current,
      style: 'https://demotiles.maplibre.org/style.json',
      center: [6.5, 45.5],
      zoom: 6,
      pitch: 60,
      bearing: 20,
      maxPitch: 85,
    })

    map.addControl(new maplibregl.NavigationControl({ visualizePitch: true }), 'top-right')
    map.dragRotate.enable()
    map.touchZoomRotate.enableRotation()

    mapRef.current = map

    map.on('load', () => {
      if (!map.getSource('terrain-dem')) {
        map.addSource('terrain-dem', {
          type: 'raster-dem',
          tiles: ['https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png'],
          tileSize: 256,
          encoding: 'terrarium',
          maxzoom: 15,
        } as any)
      }

      if (terrainEnabled) {
        map.setTerrain({ source: 'terrain-dem', exaggeration })
      }

      setIsLoaded(true)
    })

    return () => {
      mapRef.current = null
      map.remove()
    }
  }, [])

  useEffect(() => {
    const map = mapRef.current
    if (!map || !isLoaded) return

    if (terrainEnabled) {
      if (!map.getSource('terrain-dem')) {
        map.addSource('terrain-dem', {
          type: 'raster-dem',
          tiles: ['https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png'],
          tileSize: 256,
          encoding: 'terrarium',
          maxzoom: 15,
        } as any)
      }
      map.setTerrain({ source: 'terrain-dem', exaggeration })
    } else {
      map.setTerrain(null)
    }
  }, [terrainEnabled, exaggeration, isLoaded])

  return (
    <div style={{ position: 'fixed', inset: 0 }}>
      <div ref={mapContainerRef} style={{ width: '100%', height: '100%' }} />

      <div
        style={{
          position: 'absolute',
          top: 12,
          left: 12,
          zIndex: 1,
          background: 'rgba(0, 0, 0, 0.6)',
          color: '#fff',
          padding: 12,
          borderRadius: 8,
          minWidth: 220,
          boxShadow: '0 4px 12px rgba(0,0,0,0.25)',
          backdropFilter: 'blur(4px)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <input
            id="terrain-toggle"
            type="checkbox"
            checked={terrainEnabled}
            onChange={(e) => setTerrainEnabled(e.target.checked)}
          />
          <label htmlFor="terrain-toggle">Terrain</label>
        </div>
        <div style={{ marginTop: 10 }}>
          <label htmlFor="exaggeration">Exaggeration: {exaggeration.toFixed(1)}x</label>
          <input
            id="exaggeration"
            type="range"
            min={1}
            max={6}
            step={0.1}
            value={exaggeration}
            onChange={(e) => setExaggeration(Number(e.target.value))}
            disabled={!terrainEnabled}
            style={{ width: 200 }}
          />
        </div>
      </div>
    </div>
  )
}

export default App
