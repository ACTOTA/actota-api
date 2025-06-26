#!/bin/bash

# Test script to verify Vertex AI Search integration is working properly

echo "🧪 Testing ACTOTA API Search Functionality"
echo "=========================================="

BASE_URL="http://localhost:8080"

# Test 1: Search with activities but no location or dates
echo ""
echo "Test 1: Search with activities only"
echo "-----------------------------------"
response1=$(curl -s -X POST ${BASE_URL}/itineraries/search \
  -H "Content-Type: application/json" \
  -d '{
    "locations": [],
    "arrival_datetime": null,
    "departure_datetime": null,
    "adults": 2,
    "children": 0,
    "infants": 0,
    "activities": ["Adventure", "Hiking"],
    "lodging": [],
    "transportation": ""
  }')

count1=$(echo "$response1" | jq '. | length' 2>/dev/null || echo "0")
echo "Results found: $count1"

if [ "$count1" -gt 0 ]; then
    echo "✅ Test 1 PASSED - Found itineraries without location/dates"
else
    echo "❌ Test 1 FAILED - No itineraries found"
fi

# Test 2: Search with location
echo ""
echo "Test 2: Search with location"
echo "----------------------------"
response2=$(curl -s -X POST ${BASE_URL}/itineraries/search \
  -H "Content-Type: application/json" \
  -d '{
    "locations": ["Denver"],
    "arrival_datetime": null,
    "departure_datetime": null,
    "adults": 2,
    "children": 0,
    "infants": 0,
    "activities": ["Adventure"],
    "lodging": [],
    "transportation": ""
  }')

count2=$(echo "$response2" | jq '. | length' 2>/dev/null || echo "0")
echo "Results found: $count2"

if [ "$count2" -gt 0 ]; then
    echo "✅ Test 2 PASSED - Found itineraries with location"
else
    echo "❌ Test 2 FAILED - No itineraries found with location"
fi

# Test 3: Search with completely empty criteria
echo ""
echo "Test 3: Search with minimal criteria"
echo "------------------------------------"
response3=$(curl -s -X POST ${BASE_URL}/itineraries/search \
  -H "Content-Type: application/json" \
  -d '{
    "locations": [],
    "arrival_datetime": null,
    "departure_datetime": null,
    "adults": 1,
    "children": 0,
    "infants": 0,
    "activities": [],
    "lodging": [],
    "transportation": ""
  }')

count3=$(echo "$response3" | jq '. | length' 2>/dev/null || echo "0")
echo "Results found: $count3"

if [ "$count3" -gt 0 ]; then
    echo "✅ Test 3 PASSED - Found itineraries with minimal criteria"
else
    echo "❌ Test 3 FAILED - No itineraries found with minimal criteria"
fi

# Summary
echo ""
echo "📊 Test Summary"
echo "==============="

total_tests=3
passed_tests=0

if [ "$count1" -gt 0 ]; then ((passed_tests++)); fi
if [ "$count2" -gt 0 ]; then ((passed_tests++)); fi
if [ "$count3" -gt 0 ]; then ((passed_tests++)); fi

echo "Tests passed: $passed_tests/$total_tests"

if [ "$passed_tests" -eq "$total_tests" ]; then
    echo "🎉 ALL TESTS PASSED! Vertex AI Search integration is working correctly!"
else
    echo "⚠️  Some tests failed. Check your Vertex AI Search configuration."
fi

echo ""
echo "🔧 Environment Configuration"
echo "============================="
echo "Google Cloud Project: $(grep GOOGLE_CLOUD_PROJECT_ID .env.local | cut -d'=' -f2)"
echo "Vertex Search Location: $(grep VERTEX_SEARCH_LOCATION .env.local | cut -d'=' -f2)"
echo "Data Store ID: $(grep VERTEX_SEARCH_DATA_STORE_ID .env.local | cut -d'=' -f2)"
echo ""
echo "If Vertex AI Search is properly configured, you should see 'VertexSearchService initialized successfully' in the server logs."
echo ""
