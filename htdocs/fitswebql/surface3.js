let THREE = null;
let OrbitControls = null;

let container;
let scene;
let camera;
let renderer;
let controls;
let wireTexture;
let geometry;
let material;
let plane;
let surfacePoint = null;
let threeReadyPromise = null;
let initTimer = 0;

const segments = 512;
let is_active = false;

// Handle page unload/navigation to dispose resources
window.addEventListener('beforeunload', function () {
    if (is_active) {
        closeSurface();
    }
});

window.addEventListener('pagehide', function () {
    if (is_active) {
        closeSurface();
    }
});

function ensureThreeDeps() {
    if (THREE != null && OrbitControls != null) {
        return Promise.resolve();
    }

    if (threeReadyPromise != null) {
        return threeReadyPromise;
    }

    threeReadyPromise = new Promise((resolve) => {
        function attemptResolve() {
            if (window.THREE != null && window.OrbitControls != null) {
                THREE = window.THREE;
                OrbitControls = window.OrbitControls;
                surfacePoint = new THREE.Vector3();
                resolve();
                return;
            }

            setTimeout(attemptResolve, 10);
        }

        attemptResolve();
    });

    return threeReadyPromise;
}

function getMainRect() {
    return document.getElementById('mainDiv').getBoundingClientRect();
}

function getSurfaceDimensions() {
    return imageContainer[va_count - 1].image_bounding_dims;
}

function getAspectRatio() {
    const imageBoundingDims = getSurfaceDimensions();
    return imageBoundingDims.height / imageBoundingDims.width;
}

function meshFunction(x, y, target) {
    let imageCanvas = imageContainer[va_count - 1].imageCanvas;
    let imageFrame = imageContainer[va_count - 1].imageFrame;
    let imageBoundingDims = imageContainer[va_count - 1].image_bounding_dims;
    let z;

    const xcoord = Math.round(imageBoundingDims.x1 + (1 - x) * (imageBoundingDims.width - 1));
    const ycoord = Math.round(imageBoundingDims.y1 + y * (imageBoundingDims.height - 1));

    if (composite_view) {
        imageCanvas = compositeCanvas;

        const pixel = 4 * (ycoord * imageCanvas.width + xcoord);
        z = compositeImageData.data[pixel] - 127;
    } else {
        const pixel = ycoord * imageFrame.stride + xcoord;
        z = imageFrame.bytes[pixel] - 127;
    }

    const aspect = imageBoundingDims.height / imageBoundingDims.width;
    target.set(x - 0.5, (y - 0.5) * aspect, z / 2048);
}

function colourFunction(x, y) {
    let imageCanvas = imageContainer[va_count - 1].imageCanvas;
    let imageData = imageContainer[va_count - 1].imageData;
    let newImageData = imageContainer[va_count - 1].newImageData;
    let imageBoundingDims = imageContainer[va_count - 1].image_bounding_dims;

    if (composite_view) {
        imageCanvas = compositeCanvas;
        newImageData = compositeImageData;
    }

    const aspect = imageBoundingDims.height / imageBoundingDims.width;
    const xcoord = Math.round(imageBoundingDims.x1 + (1 - x - 0.5) * (imageBoundingDims.width - 1));
    const ycoord = Math.round(imageBoundingDims.y1 + (y / aspect + 0.5) * (imageBoundingDims.height - 1));
    const pixel = 4 * (ycoord * imageCanvas.width + xcoord);

    let r;
    let g;
    let b;

    if (newImageData != null) {
        r = newImageData.data[pixel];
        g = newImageData.data[pixel + 1];
        b = newImageData.data[pixel + 2];
    } else {
        r = imageData.data[pixel];
        g = imageData.data[pixel + 1];
        b = imageData.data[pixel + 2];
    }

    return new THREE.Color('rgb(' + r + ',' + g + ',' + b + ')');
}

function disposeSurfaceResources() {
    window.removeEventListener('resize', onWindowResize);

    // Clear lights from scene
    if (scene != null) {
        scene.clear();  // Removes all objects and lights from the scene
    }

    // Dispose camera
    if (camera != null) {
        camera.clear();
    }

    // Dispose mesh
    if (plane != null) {
        plane.geometry.dispose();  // Already done separately, but ensures mesh cleanup
        plane.material.dispose();  // Already done separately
    }

    if (renderer != null) {
        renderer.setAnimationLoop(null);
        renderer.dispose();
    }

    if (controls != null) {
        controls.dispose();
    }

    if (wireTexture != null) {
        wireTexture.dispose();
    }

    if (geometry != null) {
        geometry.dispose();
    }

    if (material != null) {
        material.dispose();
    }

    container = null;
    scene = null;
    camera = null;
    renderer = null;
    controls = null;
    wireTexture = null;
    geometry = null;
    material = null;
    plane = null;

    console.log('Surface resources disposed');
}

function closeSurface() {
    if (initTimer !== 0) {
        clearTimeout(initTimer);
        initTimer = 0;
    }

    is_active = false;
    disposeSurfaceResources();
    threeReadyPromise = null;  // Reset promise for next open
    d3.select('#ThreeJS').remove();
}

function buildGeometry() {
    geometry = new THREE.PlaneGeometry(1, getAspectRatio(), segments, segments);

    const position = geometry.attributes.position;
    const colors = new Float32Array(position.count * 3);

    for (let iy = 0; iy <= segments; iy++) {
        for (let ix = 0; ix <= segments; ix++) {
            const index = iy * (segments + 1) + ix;
            const x = ix / segments;
            const y = iy / segments;

            meshFunction(x, y, surfacePoint);
            position.setXYZ(index, surfacePoint.x, surfacePoint.y, surfacePoint.z);

            const color = colourFunction(surfacePoint.x, surfacePoint.y);
            colors[3 * index] = color.r;
            colors[3 * index + 1] = color.g;
            colors[3 * index + 2] = color.b;
        }
    }

    geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    geometry.computeVertexNormals();
}

function init_graph() {
    initTimer = 0;

    if (!is_active) {
        return;
    }

    const rect = getMainRect();
    const screenWidth = rect.width;
    const screenHeight = rect.height;

    scene = new THREE.Scene();

    camera = new THREE.PerspectiveCamera(45, screenWidth / screenHeight, 0.01, 100);
    camera.position.set(1.15, -1.35, 0.85);
    camera.up.set(0, 0, 1);

    renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.setSize(screenWidth, screenHeight);
    renderer.setAnimationLoop(animate_surface);

    container = document.getElementById('ThreeJS');
    container.appendChild(renderer.domElement);

    controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.enablePan = false;
    controls.minDistance = 0.35;
    controls.maxDistance = 4.0;
    controls.maxPolarAngle = Math.PI / 2;
    controls.target.set(0, 0, 0);
    controls.update();

    scene.add(new THREE.AmbientLight(0xffffff, 1.8));

    const directionalLight = new THREE.DirectionalLight(0xffffff, 1.2);
    directionalLight.position.set(1.5, -1.0, 2.0);
    scene.add(directionalLight);

    buildGeometry();

    wireTexture = new THREE.TextureLoader().load(ROOT_PATH + 'square.png');
    wireTexture.wrapS = THREE.RepeatWrapping;
    wireTexture.wrapT = THREE.RepeatWrapping;
    wireTexture.repeat.set(segments, segments);
    wireTexture.colorSpace = THREE.SRGBColorSpace;

    material = new THREE.MeshPhongMaterial({
        map: wireTexture,
        vertexColors: true,
        side: THREE.DoubleSide
    });

    plane = new THREE.Mesh(geometry, material);
    scene.add(plane);

    window.removeEventListener('resize', onWindowResize);  // Remove first to avoid duplicates
    window.addEventListener('resize', onWindowResize);
    d3.select('#hourglassThreeJS').remove();
}

function init_surface() {
    if (is_active) {
        closeSurface();
    }

    const div = d3.select('body').append('div')
        .attr('id', 'ThreeJS')
        .attr('class', 'threejs');

    div.append('span')
        .attr('id', 'closeThreeJS')
        .attr('class', 'close myclose')
        .on('click', closeSurface)
        .text('×');

    div.append('img')
        .attr('id', 'hourglassThreeJS')
        .attr('class', 'hourglass')
        .attr('src', 'https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/loading.gif')
        .attr('alt', 'hourglass')
        .style('width', 200)
        .style('height', 200);

    is_active = true;

    ensureThreeDeps().then(() => {
        if (!is_active) {
            return;
        }

        initTimer = setTimeout(init_graph, 50);
    });
}

function onWindowResize() {
    if (camera == null || renderer == null) {
        return;
    }

    const rect = getMainRect();
    camera.aspect = rect.width / rect.height;
    camera.updateProjectionMatrix();
    renderer.setSize(rect.width, rect.height);
}

function animate_surface() {
    if (!is_active || renderer == null) {
        return;
    }

    update();
    render();
}

function update() {
    if (controls != null) {
        controls.update();
    }
}

function render() {
    if (renderer != null && scene != null && camera != null) {
        renderer.render(scene, camera);
    }
}
