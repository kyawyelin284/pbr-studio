import { Suspense, useMemo } from 'react';
import { Canvas } from '@react-three/fiber';
import { OrbitControls, Environment, useTexture } from '@react-three/drei';

const HDRI_PRESETS = [
  { value: 'studio', label: 'Studio' },
  { value: 'sunset', label: 'Sunset' },
  { value: 'dawn', label: 'Dawn' },
  { value: 'night', label: 'Night' },
  { value: 'warehouse', label: 'Warehouse' },
  { value: 'forest', label: 'Forest' },
  { value: 'apartment', label: 'Apartment' },
  { value: 'city', label: 'City' },
  { value: 'park', label: 'Park' },
  { value: 'lobby', label: 'Lobby' },
] as const;

export interface TextureUrls {
  albedo?: string;
  normal?: string;
  roughness?: string;
  metallic?: string;
  ao?: string;
}

interface PBRSphereProps {
  textureUrls: TextureUrls;
}

const WHITE_PIXEL =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';

function PBRSphere({ textureUrls }: PBRSphereProps) {
  const textureMap = useMemo(() => {
    const map: Record<string, string> = {};
    map.map = textureUrls.albedo || WHITE_PIXEL;
    if (textureUrls.normal) map.normalMap = textureUrls.normal;
    if (textureUrls.roughness) map.roughnessMap = textureUrls.roughness;
    if (textureUrls.metallic) map.metalnessMap = textureUrls.metallic;
    if (textureUrls.ao) map.aoMap = textureUrls.ao;
    return map;
  }, [textureUrls]);

  const textures = useTexture(textureMap, (tex) => {
    const setFlipY = (t: unknown) => {
      if (t && typeof t === 'object' && 'flipY' in t) {
        (t as { flipY: boolean }).flipY = false;
      }
    };
    if (Array.isArray(tex)) {
      tex.forEach(setFlipY);
    } else {
      setFlipY(tex);
    }
  });

  const materialProps: Record<string, unknown> = {
    envMapIntensity: 0.8,
    roughness: textureUrls.roughness ? 1 : 0.5,
    metalness: textureUrls.metallic ? 1 : 0.2,
  };
  if ('map' in textures && textures.map) materialProps.map = textures.map;
  if ('normalMap' in textures && textures.normalMap) materialProps.normalMap = textures.normalMap;
  if ('roughnessMap' in textures && textures.roughnessMap) materialProps.roughnessMap = textures.roughnessMap;
  if ('metalnessMap' in textures && textures.metalnessMap) materialProps.metalnessMap = textures.metalnessMap;
  if ('aoMap' in textures && textures.aoMap) {
    materialProps.aoMap = textures.aoMap;
    materialProps.aoMapIntensity = 1;
  }

  return (
    <mesh castShadow receiveShadow>
      <sphereGeometry args={[1, 64, 64]} />
      <meshStandardMaterial {...materialProps} />
    </mesh>
  );
}

function Scene({
  textureUrls,
  hdriPreset,
}: {
  textureUrls: TextureUrls;
  hdriPreset: string;
}) {
  return (
    <>
      <ambientLight intensity={0.3} />
      <directionalLight position={[5, 5, 5]} intensity={0.8} castShadow />
      <directionalLight position={[-3, 3, 2]} intensity={0.4} />
      <PBRSphere textureUrls={textureUrls} />
      <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, -1.5, 0]} receiveShadow>
        <planeGeometry args={[10, 10]} />
        <meshStandardMaterial color="#2a2a2a" roughness={0.8} metalness={0} />
      </mesh>
      <Environment preset={hdriPreset as 'studio' | 'sunset' | 'dawn' | 'night' | 'warehouse' | 'forest' | 'apartment' | 'city' | 'park' | 'lobby'} />
    </>
  );
}

interface Viewport3DProps {
  textureUrls?: TextureUrls;
  textureUrlsB?: TextureUrls;
  hdriPreset?: string;
  onPresetChange?: (preset: string) => void;
  compareMode?: boolean;
  labelA?: string;
  labelB?: string;
  /** Increment to force texture reload (e.g. when files change) */
  refreshKey?: number;
}

function SingleViewport({
  textureUrls,
  hdriPreset,
  label,
}: {
  textureUrls: TextureUrls;
  hdriPreset: string;
  label?: string;
}) {
  return (
    <div className="viewport-single">
      {label && <div className="viewport-label">{label}</div>}
      <Canvas
        shadows
        camera={{ position: [0, 0, 4], fov: 45 }}
        gl={{ antialias: true }}
      >
        <Suspense fallback={null}>
          <Scene textureUrls={textureUrls} hdriPreset={hdriPreset} />
          <OrbitControls enableDamping dampingFactor={0.05} />
        </Suspense>
      </Canvas>
    </div>
  );
}

export function Viewport3D({
  textureUrls = {},
  textureUrlsB,
  hdriPreset = 'studio',
  onPresetChange,
  compareMode = false,
  labelA = 'A',
  labelB = 'B',
  refreshKey = 0,
}: Viewport3DProps) {
  const showCompare = compareMode && textureUrlsB != null;

  return (
    <div className="panel panel-center">
      <div className="panel-header viewport-header">
        <span>{showCompare ? 'Material Comparison' : '3D Preview'}</span>
        <div className="hdri-selector">
          <label htmlFor="hdri-preset">HDRI: </label>
          <select
            id="hdri-preset"
            value={hdriPreset}
            onChange={(e) => onPresetChange?.(e.target.value)}
          >
            {HDRI_PRESETS.map((p) => (
              <option key={p.value} value={p.value}>
                {p.label}
              </option>
            ))}
          </select>
        </div>
      </div>
      <div className={`viewport ${showCompare ? 'viewport-compare' : ''}`}>
        {showCompare ? (
          <>
            <SingleViewport
              key={`a-${refreshKey}`}
              textureUrls={textureUrls}
              hdriPreset={hdriPreset}
              label={labelA}
            />
            <div className="viewport-divider" />
            <SingleViewport
              key={`b-${refreshKey}`}
              textureUrls={textureUrlsB}
              hdriPreset={hdriPreset}
              label={labelB}
            />
          </>
        ) : (
          <Canvas
            key={refreshKey}
            shadows
            camera={{ position: [0, 0, 4], fov: 45 }}
            gl={{ antialias: true }}
          >
            <Suspense fallback={null}>
              <Scene textureUrls={textureUrls} hdriPreset={hdriPreset} />
              <OrbitControls enableDamping dampingFactor={0.05} />
            </Suspense>
          </Canvas>
        )}
      </div>
    </div>
  );
}
