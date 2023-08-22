# Default libraries
import json
import time
import os

# For browsing
import selenium
from selenium import webdriver
from selenium.webdriver.firefox.service import Service
from selenium.webdriver.firefox.options import Options
from selenium.webdriver.common.by import By


url = input("Enter the url of Pinterest board: ")

# Add a directory
img_directory = input('Enter new or existing directory for images: ')
try:
    os.mkdir(img_directory)
    print('Creating new directory for pictures.')
except:
    print('Adding pictures to existing directory')

# Firefox driver
driver_path = './geckodriver'
driver = webdriver.Firefox(
    service=Service(driver_path),
    options=Options()
)

# Get email and password from json
with open('info.json') as info_file:
    info = json.load(info_file)


# Open login window
driver.get(url)
login_button = driver.find_element(By.CSS_SELECTOR, "div[data-test-id='login-button']")
login_button.click()

# Find and fill the fields
email_field = driver.find_element(By.ID, 'email')
email_field.send_keys(info['login'])
password_field = driver.find_element(By.ID, 'password')
password_field.send_keys(info['pass'])

# Click login button again
login_button = driver.find_element(By.CLASS_NAME, 'SignupButton')
login_button.click()


# Prepare for main loop
new_img_not_met = 0 # stopping flag
urls = set()        # store urls that already downloaded for avoid duplication
i = 0               # counter for debug information

# if the directory already existed and there are already
# downloaded pictures, no need to redownloaded them again
if ("downloads.txt" in os.listdir(img_directory)):
    with open(f"{img_directory}/downloads.txt") as downloaded_list_file:
        urls = set([line[:-1].rstrip() for line in downloaded_list_file])
# main loop
while True:
    # Get all images available
    images = driver.find_elements(By.CSS_SELECTOR, 'img[srcset]')

    # Process every image found
    for img in images:
        
        # Take the best resolution for a picture.
        try:
            attribute = img.get_attribute('srcset')
            if (len(attribute.split(',')) == 1):
                # Remove ' 4x' substring
                current_url = attribute.split()[0]
            else:
                # In case other versions exist
                current_url = img.get_attribute('srcset').split(',')[-1].split()[0]
        except:
            continue


        # If there was no such urls met
        if current_url not in urls:
            new_img_not_met = 0

            # Scroll to that image
            driver.execute_script('arguments[0].scrollIntoView({block: "center", behavior: "smooth"});', img)

            # Add to the set
            urls.add(current_url)
            with open(f'{img_directory}/downloads.txt', 'a') as downloads:
                downloads.write(f'{current_url}\n')
            
            print(f'{i} - {current_url}')
            i += 1

        else:
            # if no image met, we increment the counter
            # for stopping condition
            new_img_not_met += 1

    if new_img_not_met > 10:
        break

print('Done!')