#!/bin/bash

# Path to your HTML file
html_file="test.html"

# Base position for the animation
base_left=0 # Starting left position in pixels
base_top=0 # Starting top position in pixels

# Animation parameters
amplitude=100 # Amplitude of the sine wave in pixels
period=10 # Period of the sine wave in seconds

while true; do
  # Calculate the current time in the cycle
  current_time=$(date +%s)
  time_in_cycle=$(($current_time % $period))

  # Sine wave calculation for left movement
  radians=$(echo "scale=4; $time_in_cycle/$period*2*a(1)" | bc -l)
  sine_wave=$(echo "s($radians)" | bc -l)
  new_left=$(echo "$base_left + $sine_wave*$amplitude" | bc)

  # Calculate the new top position to simulate falling
  new_top=$(($base_top + ($time_in_cycle * 5))) # Increase for faster falling

  # Limit the top position to restart the animation
  if [ $new_top -ge 300 ]; then # Example limit, adjust as needed
    base_top=0
  else
    base_top=$new_top
  fi

  # Generate the new class names with arbitrary values
  top_class="top-[$new_top""px]"
  left_class="left-[$new_left""px]"

  # Construct the new HTML tag with updated positions
  new_img_tag="<img id=\"snowflake\" class=\"$top_class $left_class absolute w-full h-full\" src=\"https://www.pngall.com/wp-content/uploads/13/Snowflake-PNG-Image-File.png\"></img>"

  # Use sed to replace the existing snowflake div in the HTML file
  # This example assumes you can uniquely identify the snowflake div, e.g., by an id="snowflake"
  sed -i '' "s|<img id=\"snowflake\".*</img>|$new_img_tag|" $html_file

  # Wait before updating again
  sleep 0.01
done