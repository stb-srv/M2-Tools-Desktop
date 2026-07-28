import { useEffect, useRef } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import type { Gr2ModelInfo } from "./types";

interface SceneRefs {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  renderer: THREE.WebGLRenderer;
  controls: OrbitControls;
  meshGroup: THREE.Group;
}

interface Gr2CanvasProps {
  model: Gr2ModelInfo | null;
  className?: string;
}

export function Gr2Canvas({ model, className }: Gr2CanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<SceneRefs | null>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x1e1e1e);

    const camera = new THREE.PerspectiveCamera(
      50,
      container.clientWidth / Math.max(container.clientHeight, 1),
      0.1,
      10000,
    );
    camera.position.set(100, 100, 200);

    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(container.clientWidth, container.clientHeight);
    container.appendChild(renderer.domElement);

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;

    scene.add(new THREE.AmbientLight(0xffffff, 0.6));
    const directional = new THREE.DirectionalLight(0xffffff, 1.2);
    directional.position.set(200, 300, 200);
    scene.add(directional);
    scene.add(new THREE.GridHelper(200, 20, 0x444444, 0x2a2a2a));

    // Granny/Metin2 models are Z-up, three.js is Y-up - without this every
    // model shows up lying on its back.
    const meshGroup = new THREE.Group();
    meshGroup.rotation.x = -Math.PI / 2;
    scene.add(meshGroup);

    sceneRef.current = { scene, camera, renderer, controls, meshGroup };

    let frameId = 0;
    const animate = () => {
      frameId = requestAnimationFrame(animate);
      controls.update();
      renderer.render(scene, camera);
    };
    animate();

    const resizeObserver = new ResizeObserver(() => {
      if (container.clientWidth === 0 || container.clientHeight === 0) return;
      camera.aspect = container.clientWidth / container.clientHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(container.clientWidth, container.clientHeight);
    });
    resizeObserver.observe(container);

    return () => {
      cancelAnimationFrame(frameId);
      resizeObserver.disconnect();
      controls.dispose();
      renderer.dispose();
      container.removeChild(renderer.domElement);
      sceneRef.current = null;
    };
  }, []);

  useEffect(() => {
    const refs = sceneRef.current;
    if (!refs) return;
    const { camera, controls, meshGroup } = refs;

    for (const child of [...meshGroup.children]) {
      meshGroup.remove(child);
      if (child instanceof THREE.Mesh) {
        child.geometry.dispose();
        const material = child.material as THREE.MeshPhongMaterial;
        material.map?.dispose();
        material.dispose();
      }
    }

    if (!model || model.meshes.length === 0) return;

    const loader = new THREE.TextureLoader();

    for (const mesh of model.meshes) {
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute(
        "position",
        new THREE.Float32BufferAttribute(mesh.vertices, 3),
      );
      geometry.setAttribute(
        "normal",
        new THREE.Float32BufferAttribute(mesh.normals, 3),
      );
      geometry.setAttribute("uv", new THREE.Float32BufferAttribute(mesh.uvs, 2));
      geometry.setIndex(new THREE.Uint32BufferAttribute(mesh.indices, 1));

      const material = new THREE.MeshPhongMaterial({
        // Textured meshes need a white base or the texture gets tinted.
        color: mesh.texture ? 0xffffff : 0xcfcfcf,
        side: THREE.DoubleSide,
      });

      if (mesh.texture) {
        const texture = loader.load(mesh.texture);
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.flipY = false;
        material.map = texture;
        // Metin2 textures use alpha for cutouts (hair, capes, straps).
        material.transparent = true;
        material.alphaTest = 0.5;
        material.needsUpdate = true;
      }

      meshGroup.add(new THREE.Mesh(geometry, material));
    }

    const box = new THREE.Box3().setFromObject(meshGroup);
    const center = box.getCenter(new THREE.Vector3());
    const size = Math.max(box.getSize(new THREE.Vector3()).length(), 1);
    meshGroup.position.sub(center);
    camera.position.set(size * 0.5, size * 0.5, size * 0.8);
    camera.near = size / 100;
    camera.far = size * 100;
    camera.updateProjectionMatrix();
    controls.target.set(0, 0, 0);
    controls.update();
  }, [model]);

  return <div ref={containerRef} className={className} />;
}
