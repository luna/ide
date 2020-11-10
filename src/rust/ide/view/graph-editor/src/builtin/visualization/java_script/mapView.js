function loadScript(url) {
    var script = document.createElement("script");
    script.src = url;

    document.head.appendChild(script);
}

function loadStyle(url) {
    var link  = document.createElement("link");
    link.href = url;
    link.rel  = 'stylesheet';

    document.head.appendChild(link);
}

loadScript('https://unpkg.com/deck.gl@latest/dist.min.js');
loadScript('https://api.tiles.mapbox.com/mapbox-gl-js/v1.6.1/mapbox-gl.js');
loadStyle('https://api.tiles.mapbox.com/mapbox-gl-js/v1.6.1/mapbox-gl.css');

const styleHead = document.createElement("style")
styleHead.innerText = `.mapboxgl-map {
            border-radius: 14px;
        }`
document.head.appendChild(styleHead);

const TOKEN = 'pk.eyJ1IjoiZW5zby1vcmciLCJhIjoiY2tmNnh5MXh2MGlyOTJ5cWdubnFxbXo4ZSJ9.3KdAcCiiXJcSM18nwk09-Q';

const GEO_POINT         = "GeoPoint";
const GEO_MAP           = "GeoMap";
const SCATTERPLOT_LAYER = "ScatterplotLayer";
/**
 * Provides a mapbox & deck.gl-based map visualization for IDE.
 *
 * > Example creates a map with described properties with a scatter plot overlay.
 * {
 * "type": "GeoMap",
 * "latitude": 37.8,
 * "longitude": -122.45,
 * "zoom": 15,
 * "controller": true,
 * "layers": [{
 *     "type": "ScatterplotLayer",
 *     "data": [{
 *         "type": "GeoPoint",
 *         "latitude": -122.45,
 *         "longitude": 37.8,
 *         "color": [255, 0, 0],
 *         "radius": 100
 *     }]
 * }]
 * }
 */
class MapViewVisualization extends Visualization {
    static inputType = "Any"

    onDataReceived(data) {

        while (this.dom.firstChild) {
            this.dom.removeChild(this.dom.lastChild);
        }

        const width   = this.dom.getAttributeNS(null, "width");
        const height  = this.dom.getAttributeNS(null, "height");
        const mapElem = document.createElement("div");
        mapElem.setAttributeNS(null,"id"   , "map");
        mapElem.setAttributeNS(null,"style","width:" + width + "px;height: " + height + "px;");
        this.dom.appendChild(mapElem);

        let parsedData = data;
        if (typeof data === "string") {
            parsedData = JSON.parse(data);
        }

        let defaultMapStyle = 'mapbox://styles/mapbox/light-v9';
        let accentColor     = [1,234,146];
        let defaultRadius   = 150;
        if (document.getElementById("root").classList.contains("dark")){
            defaultMapStyle = 'mapbox://styles/mapbox/dark-v9';
            accentColor     = [222,162,47];
        }

        let preparedDataPoints = []
        if (parsedData.type === GEO_POINT) {
            let radius = isNaN(x.radius) ? defaultRadius : x.radius;
            preparedDataPoints.push({
                position:[parsedData.longitude,parsedData.latitude],
                color:parsedData.color || accentColor,
                radius:radius || defaultRadius
            });
        } else if (Array.isArray(parsedData) && parsedData[0].type === GEO_POINT) {
            parsedData.forEach(dataPoint => {
                let radius = isNaN(x.radius) ? defaultRadius : x.radius;
                preparedDataPoints.push({
                    position:[dataPoint.longitude,dataPoint.latitude],
                    color:dataPoint.color || accentColor,
                    radius:radius || defaultRadius
                });
            })
        } else {
            if (parsedData.layers !== undefined) {
                parsedData.layers.forEach(layer => {
                    if (layer.type === SCATTERPLOT_LAYER) {
                        let dataPoints = layer.data || [];
                        dataPoints.forEach(x => {
                            let radius = isNaN(x.radius) ? defaultRadius : x.radius;
                            preparedDataPoints.push({
                                position:[x.longitude,x.latitude],
                                color:x.color || accentColor,
                                radius:radius
                            })
                        });
                    } else {
                        console.log("Currently unsupported deck.gl layer.")
                    }
                })
            }
        }

        const scatterplotLayer = new deck.ScatterplotLayer({
            data: preparedDataPoints,
            getFillColor: d => d.color,
            getRadius: d => d.radius
        })

        //TODO: Compute lat/lon/zoom if ther types are "null" (1,9)
        //      Refactor a little?

        const deckgl = new deck.DeckGL({
            container: 'map',
            mapboxApiAccessToken: TOKEN,
            mapStyle: parsedData.mapStyle || defaultMapStyle,
            initialViewState: {
                longitude: parsedData.longitude || 0.0,
                latitude: parsedData.latitude || 0.0,
                zoom: parsedData.zoom || 11,
                pitch: parsedData.pitch || 0
            },
            controller: parsedData.controller || true
        });

        deckgl.setProps({
            layers: [scatterplotLayer]
        });
    }

    setSize(size) {
        this.dom.setAttributeNS(null, "width", size[0]);
        this.dom.setAttributeNS(null, "height", size[1]);
    }
}

return MapViewVisualization;
