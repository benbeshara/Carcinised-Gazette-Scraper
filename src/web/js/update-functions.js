function updatePolygons(geojsonData) {
    currentPolygons.forEach(function(polygon) {
        polygon.remove();
    });
    currentPolygons = [];

    L.geoJSON(geojsonData, {
        style: function(feature) {
            let start = Date.parse(feature.properties.start);
            let end = Date.parse(feature.properties.end);
            let date = Date.now()

            if((start < date < end) || (date === start)){
                return {
                    color: '#ff0000',
                    weight: 2,
                    opacity: 0.65
                }
            }
            if(date > end){
                return {
                    color: '#4444ff',
                    weight: 2,
                    opacity: 0.25
                }
            }
            return {
                color: '#ffec16',
                weight: 2,
                opacity: 0.65
            };
        },
        onEachFeature: function(feature, layer) {
            // Create popup content using feature properties
            const popupContent = `
                                    <div class='custom-popup'>
                                        <h4>${feature.properties.title || 'No Title'}</h4>
                                        <p><a href='${feature.properties.uri}' target='_blank'>View Details</a></p>
                                        ${feature.properties.img_uri ?
                `<img alt='${feature.properties.title}' src='${feature.properties.img_uri}' style='max-width: 200px;'>`
                : ''}
                                    </div>
                                `;
            layer.bindPopup(popupContent);
            currentPolygons.push(layer);
        }
    }).addTo(map);
}

function updateCircles(geojsonData) {
    console.log('GeoJSON data:', geojsonData);
    currentCircles.forEach(function(circle) {
        circle.remove();
    });
    currentCircles = [];
    console.log(geojsonData);

    L.geoJSON(geojsonData, {
        pointToLayer: (feature, latlng) => {
            return new L.Circle(latlng, {radius: 500});
        },
        style: function(feature) {
            let start = Date.parse(feature.properties.start);
            let end = Date.parse(feature.properties.end);
            let date = Date.now()

            if((start < date < end) || (date === start)){
                return {
                    color: '#ff0000',
                    weight: 2,
                    opacity: 0.65
                }
            }
            if(date > end){
                return {
                    color: '#4444ff',
                    weight: 2,
                    opacity: 0.25
                }
            }
            return {
                color: '#ffec16',
                weight: 2,
                opacity: 0.65
            };
        },
        onEachFeature: function(feature, layer) {
            // Create popup content using feature properties
            const popupContent = `
                                    <div class='custom-popup'>
                                        <h4>${feature.properties.title || 'No Title'}</h4>
                                        <p><a href='${feature.properties.uri}' target='_blank'>View Details</a></p>
                                        ${feature.properties.img_uri ?
                `<img alt='${feature.properties.title}' src='${feature.properties.img_uri}' style='max-width: 200px;'>`
                : ''}
                                    </div>
                                `;
            layer.bindPopup(popupContent);
            currentCircles.push(layer);
        }
    }).addTo(map);
}
